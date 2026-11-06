// External scanner for the one token the grammar cannot decide on its own: the
// `?` that propagates a failure, which is spelled like the `?` that opens a
// ternary. Telling `f(x)? - 1` from `c ? -1 : 2` needs to look past the `?` for
// the ternary's `:`, which is unbounded lookahead. The compiler settles it with
// a speculative parse; this scanner mirrors the same rule on the raw text:
//
// 1. A `?` whose next character cannot begin an expression is a try, since a
//    ternary branch must start with one: `f(x)?;`, `f(x)? * 2`, `f(x)? : 0`.
// 2. A `?` attached to its operand and followed by a spaced `+` or `-` is a try
//    followed by a binary operator: `f(x)? - 1`. This is the one place the
//    scanner uses spacing. The compiler reads the branch to find out, and on
//    `c ? f(x)? - 1 : 2` only the outer ternary's context shows that the inner
//    `?` is a try; the scanner has no such context, and formatted code always
//    spaces a binary operator and never a unary sign (`c ? -1 : 2`).
// 3. Otherwise the text after it is scanned at bracket depth zero. Reaching the
//    ternary's `:` makes it a ternary; reaching a terminator first (`;`, `,`, a
//    closing bracket that was never opened, or the end of the input) makes it a
//    try. A nested `?` that itself opens a ternary owes a `:` of its own, and the
//    first two rules decide whether it does.
//
// Returning false leaves the character to the grammar's own `?` token, which is
// the ternary's; returning true emits the try's.
//
// The `?` of an optional arrow parameter is external for the same reason. After
// `(a` the parser keeps a parameter reading and an expression reading alive, and
// a token the scanner emits for one of them would starve the other. So when both
// are possible the scanner picks: `(a?: i32)` is a parameter, and `(a?, b)`,
// `(a?)` and `(a? = 1)` are parameters only when the closing `)` is followed by
// `=>` or by the `:` of a return type.
//
// The `?` of `value as T?`, a fallible cast, is external too, and wins whenever
// it is possible and touches the type, as the compiler reads it. `(value as T)?`
// is how a try on a cast is written.

#include "tree_sitter/parser.h"

#include <stdbool.h>
#include <stdint.h>

enum TokenType {
  TRY_QUESTION,
  OPTIONAL_PARAMETER_QUESTION,
  FALLIBLE_AS_QUESTION,
};

// Past this many characters the scan gives up and leaves the `?` to the
// ternary, the reading the grammar had before this scanner existed.
#define MAX_LOOKAHEAD 8192

// ASCII on purpose: the grammar's identifiers and numbers are ASCII, and editors
// that load the grammar as WebAssembly (Zed) only provide a small C library, so
// the scanner imports no function at all.
static bool is_space(int32_t c) {
  return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\f' || c == '\v';
}

static bool is_word(int32_t c) {
  return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
}

void *tree_sitter_tyrus_external_scanner_create(void) { return NULL; }

void tree_sitter_tyrus_external_scanner_destroy(void *payload) { (void)payload; }

unsigned tree_sitter_tyrus_external_scanner_serialize(void *payload, char *buffer) {
  (void)payload;
  (void)buffer;
  return 0;
}

void tree_sitter_tyrus_external_scanner_deserialize(void *payload, const char *buffer,
                                                   unsigned length) {
  (void)payload;
  (void)buffer;
  (void)length;
}

static void advance(TSLexer *lexer) { lexer->advance(lexer, false); }

static bool at_end(TSLexer *lexer) { return lexer->eof(lexer); }

static bool can_start_expression(int32_t c) {
  return is_word(c) || c == '$' || c == '"' || c == '\'' || c == '`' ||
         c == '(' || c == '[' || c == '{' || c == '-' || c == '+' || c == '!' || c == '~' ||
         c == '&' || c == '/' || c == '%';
}

static void skip_spaces(TSLexer *lexer, unsigned *budget) {
  while (!at_end(lexer) && *budget > 0) {
    if (is_space(lexer->lookahead)) {
      advance(lexer);
      (*budget)--;
      continue;
    }
    return;
  }
}

static void skip_quoted(TSLexer *lexer, int32_t quote, unsigned *budget) {
  advance(lexer);
  while (!at_end(lexer) && *budget > 0) {
    int32_t c = lexer->lookahead;
    advance(lexer);
    (*budget)--;
    if (c == '\\') {
      if (!at_end(lexer)) {
        advance(lexer);
        (*budget)--;
      }
      continue;
    }
    if (c == quote) {
      return;
    }
  }
}

// Called with the lexer on the first character after a `?` and its spaces, and
// told whether the `?` touched its operand. True when that `?` is followed by a
// spaced `+` or `-`, the shape of a binary operator after a try (rule 2).
static bool binary_sign_follows(TSLexer *lexer, bool attached, unsigned *budget) {
  int32_t sign = lexer->lookahead;
  if (!attached || (sign != '+' && sign != '-')) {
    return false;
  }
  advance(lexer);
  (*budget)--;
  return is_space(lexer->lookahead);
}

// Called inside a parenthesized list, after a `?` and its spaces. True when the
// list closes and is followed by `=>` or a return type's `:`, which makes it an
// arrow function's parameters.
static bool closes_arrow_parameters(TSLexer *lexer, unsigned *budget) {
  unsigned depth = 0;
  while (!at_end(lexer) && *budget > 0) {
    int32_t c = lexer->lookahead;
    if (c == '"' || c == '\'' || c == '`') {
      skip_quoted(lexer, c, budget);
      continue;
    }
    if (c == '(' || c == '[' || c == '{') {
      depth++;
    } else if (c == ']' || c == '}' || c == ';') {
      if (depth == 0) {
        return false;
      }
      if (c != ';') {
        depth--;
      }
    } else if (c == ')') {
      if (depth == 0) {
        advance(lexer);
        (*budget)--;
        skip_spaces(lexer, budget);
        if (lexer->lookahead == ':') {
          return true;
        }
        if (lexer->lookahead != '=') {
          return false;
        }
        advance(lexer);
        return lexer->lookahead == '>';
      }
      depth--;
    }
    advance(lexer);
    (*budget)--;
  }
  return false;
}

// Called with the lexer on the character after a `?`. Decides whether that `?`
// reaches a `:` of its own before the expression it belongs to ends.
static bool reaches_ternary_colon(TSLexer *lexer) {
  unsigned budget = MAX_LOOKAHEAD;
  unsigned depth = 0;
  unsigned owed_colons = 1;
  bool after_space = false;
  while (!at_end(lexer) && budget > 0) {
    int32_t c = lexer->lookahead;
    switch (c) {
      case '"':
      case '\'':
      case '`':
        skip_quoted(lexer, c, &budget);
        after_space = false;
        continue;
      case '/':
        advance(lexer);
        budget--;
        if (lexer->lookahead == '/') {
          while (!at_end(lexer) && lexer->lookahead != '\n' && budget > 0) {
            advance(lexer);
            budget--;
          }
        } else if (lexer->lookahead == '*') {
          int32_t previous = 0;
          advance(lexer);
          while (!at_end(lexer) && budget > 0) {
            int32_t current = lexer->lookahead;
            advance(lexer);
            budget--;
            if (previous == '*' && current == '/') {
              break;
            }
            previous = current;
          }
        }
        after_space = false;
        continue;
      case '(':
      case '[':
      case '{':
        depth++;
        break;
      case ')':
      case ']':
      case '}':
        if (depth == 0) {
          return false;
        }
        depth--;
        break;
      case ';':
      case ',':
        if (depth == 0) {
          return false;
        }
        break;
      case ':':
        if (depth == 0) {
          owed_colons--;
          if (owed_colons == 0) {
            return true;
          }
        }
        break;
      case '?':
        if (depth == 0) {
          bool attached = !after_space;
          advance(lexer);
          budget--;
          after_space = false;
          int32_t next = lexer->lookahead;
          if (next == '?' || next == '.') {
            advance(lexer);
            budget--;
            continue;
          }
          skip_spaces(lexer, &budget);
          if (can_start_expression(lexer->lookahead) &&
              !binary_sign_follows(lexer, attached, &budget)) {
            owed_colons++;
          }
          continue;
        }
        break;
      default:
        break;
    }
    after_space = is_space(c);
    advance(lexer);
    budget--;
  }
  return false;
}

bool tree_sitter_tyrus_external_scanner_scan(void *payload, TSLexer *lexer,
                                            const bool *valid_symbols) {
  (void)payload;
  bool try_valid = valid_symbols[TRY_QUESTION];
  bool parameter_valid = valid_symbols[OPTIONAL_PARAMETER_QUESTION];
  bool fallible_valid = valid_symbols[FALLIBLE_AS_QUESTION];
  if (!try_valid && !parameter_valid && !fallible_valid) {
    return false;
  }
  bool attached = true;
  while (is_space(lexer->lookahead)) {
    lexer->advance(lexer, true);
    attached = false;
  }
  if (lexer->lookahead != '?') {
    return false;
  }
  advance(lexer);
  // `??` and `?.` are tokens of their own.
  if (lexer->lookahead == '?' || lexer->lookahead == '.') {
    return false;
  }
  lexer->mark_end(lexer);
  if (fallible_valid && attached) {
    lexer->result_symbol = FALLIBLE_AS_QUESTION;
    return true;
  }
  if (!try_valid && !parameter_valid) {
    return false;
  }
  if (!try_valid) {
    lexer->result_symbol = OPTIONAL_PARAMETER_QUESTION;
    return true;
  }
  lexer->result_symbol = TRY_QUESTION;

  unsigned budget = MAX_LOOKAHEAD;
  skip_spaces(lexer, &budget);
  if (parameter_valid) {
    int32_t next = lexer->lookahead;
    if (next == ':') {
      lexer->result_symbol = OPTIONAL_PARAMETER_QUESTION;
      return true;
    }
    if (next == ',' || next == ')' || next == '=') {
      if (closes_arrow_parameters(lexer, &budget)) {
        lexer->result_symbol = OPTIONAL_PARAMETER_QUESTION;
      }
      return true;
    }
  }
  if (at_end(lexer) || !can_start_expression(lexer->lookahead)) {
    return true;
  }
  if (binary_sign_follows(lexer, attached, &budget)) {
    return true;
  }
  return !reaches_ternary_colon(lexer);
}
