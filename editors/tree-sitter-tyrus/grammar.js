/**
 * tree-sitter grammar for tyrus.
 *
 * Covers the syntax exercised by the language; semantic detail (types,
 * ownership, diagnostics) comes from the tyrus language server, so this grammar
 * only needs to describe structure for highlighting and folding.
 */

const PREC = {
  assign: 1,
  pipeline: 2,
  ternary: 3,
  nullish: 4,
  or: 5,
  and: 6,
  bit_or: 7,
  bit_xor: 8,
  bit_and: 9,
  equality: 10,
  in: 11,
  relational: 12,
  range: 13,
  shift: 14,
  additive: 15,
  multiplicative: 16,
  exponent: 17,
  as: 18,
  unary: 19,
  postfix: 20,
  call: 21,
  member: 22,
};

module.exports = grammar({
  name: 'tyrus',

  word: $ => $.identifier,

  extras: $ => [/\s/, $.comment],

  // The `?` that propagates a failure, and the `?` of an optional arrow
  // parameter. Both are spelled like the ternary's `?`, and telling them apart
  // needs lookahead past the `?`, so src/scanner.c decides; see the rules at the
  // top of that file. The parameter's `?` has to be external too: `(a?: i32)`
  // keeps an arrow-parameter reading and an expression reading alive at once,
  // and the scanner can only pick for both if it sees both as possible. The `?`
  // that makes `as T?` a fallible cast is external for the same reason.
  externals: $ => [$._try_question, $._optional_parameter_question, $._fallible_as_question],

  conflicts: $ => [
    [$.block, $.object],
    [$.parameter, $.primary_expression],
    [$.array_pattern, $.array_literal],
    [$.object_pattern, $.object],
    [$.array_pattern, $.primary_expression],
    [$.object, $.primary_expression],
    [$.behavior_interface_body, $.object_type],
    [$.parenthesized_type, $._function_type_parameter],
  ],

  supertypes: $ => [$._statement, $._expression, $._type, $.primary_expression],

  rules: {
    source_file: $ => repeat($._statement),

    comment: _ =>
      token(
        choice(
          seq('//', /[^\n]*/),
          seq('/*', /[^*]*\*+([^/*][^*]*\*+)*/, '/'),
        ),
      ),

    // ── Statements ──────────────────────────────────────────────────────────
    _statement: $ =>
      choice(
        $.import_statement,
        $.export_statement,
        $.decorated_declaration,
        $.declaration,
        $.if_statement,
        $.for_statement,
        $.while_statement,
        $.do_statement,
        $.switch_statement,
        $.match_statement,
        $.return_statement,
        $.break_statement,
        $.continue_statement,
        $.throw_statement,
        $.try_statement,
        $.block,
        $.expression_statement,
      ),

    import_statement: $ =>
      seq(
        'import',
        choice(
          // import name from "./m"  /  import name, { a } from "./m"
          seq(
            field('default', $.identifier),
            optional(seq(',', choice($.namespace_import, $.named_imports))),
          ),
          $.namespace_import,
          $.named_imports,
        ),
        'from',
        field('source', $.string),
        optional(';'),
      ),

    namespace_import: $ => seq('*', 'as', field('alias', $.identifier)),

    named_imports: $ =>
      seq('{', commaSep($.import_specifier), optional(','), '}'),

    import_specifier: $ =>
      seq(
        optional('type'),
        field('name', choice($.identifier, 'default')),
        optional(seq('as', field('alias', $.identifier))),
      ),

    export_statement: $ =>
      seq(
        'export',
        optional('protect'),
        choice(
          $.declaration,
          $.decorated_declaration,
          seq(
            'default',
            choice($.declaration, seq(field('name', $.identifier), optional(';'))),
          ),
          $.export_clause,
          seq('*', 'from', field('source', $.string), optional(';')),
        ),
      ),

    export_clause: $ =>
      seq(
        '{',
        commaSep($.export_specifier),
        optional(','),
        '}',
        optional(seq('from', field('source', $.string))),
        optional(';'),
      ),

    export_specifier: $ =>
      seq(
        field('name', choice($.identifier, 'default')),
        optional(seq('as', field('alias', choice($.identifier, 'default')))),
      ),

    decorated_declaration: $ => seq(repeat1($.decorator), optional('export'), $.declaration),

    declaration: $ =>
      choice(
        $.variable_declaration,
        $.comptime_arrow_declaration,
        $.function_declaration,
        $.behavior_interface_declaration,
        $.interface_declaration,
        $.type_alias_declaration,
        $.enum_declaration,
      ),

    variable_declaration: $ =>
      seq(
        optional('readonly'),
        optional('validator'),
        choice('const', 'let'),
        field('name', choice($.identifier, $.array_pattern, $.object_pattern)),
        optional(seq(':', field('type', $._type))),
        optional(
          seq('=', field('value', $._expression), optional(field('onfail', $.onfail_clause))),
        ),
        optional(';'),
      ),

    // Handles a failing initializer in place, the same place the compiler accepts
    // it: after the value of a declaration, with a block or a fallback expression.
    onfail_clause: $ => seq('onfail', choice($.block, $._expression)),

    comptime_arrow_declaration: $ =>
      seq(
        'comptime',
        'const',
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        '=',
        field('value', $.arrow_function),
        optional(';'),
      ),

    array_pattern: $ => seq('[', commaSep($.identifier), ']'),
    object_pattern: $ => seq('{', commaSep($.identifier), '}'),

    function_declaration: $ =>
      seq(
        optional(choice('validator', 'async', 'comptime')),
        'function',
        field('name', $.identifier),
        optional($.type_parameters),
        field('parameters', $.parameters),
        optional(seq(':', field('return_type', $._type))),
        optional(seq('throws', field('throws_type', $._type))),
        field('body', $.block),
      ),

    interface_declaration: $ =>
      seq(
        'interface',
        field('name', $.identifier),
        optional($.type_parameters),
        optional(seq('extends', commaSep(choice($.generic_type, $.type_identifier)))),
        field('body', $.object_type),
      ),

    behavior_interface_declaration: $ =>
      seq(
        'interface',
        field('name', $.identifier),
        optional($.type_parameters),
        'at',
        field('targets', commaSep($._type)),
        optional(seq('extends', commaSep(choice($.generic_type, $.type_identifier)))),
        field('body', $.behavior_interface_body),
      ),

    behavior_interface_body: $ =>
      seq('{', repeat(choice($.behavior_method, $.behavior_binding)), '}'),

    behavior_method: $ =>
      seq(
        field('name', $.identifier),
        optional($.type_parameters),
        field('parameters', $.parameters),
        optional(seq(':', field('return_type', $._type))),
        optional(seq('throws', field('throws_type', $._type))),
        field('body', $.block),
      ),

    behavior_binding: $ => seq(field('name', $.identifier), optional(';')),

    // The `;` is required, as the compiler requires it, which is also what keeps
    // an intersection from closing the declaration at its first operand.
    type_alias_declaration: $ =>
      seq(
        'type',
        field('name', $.identifier),
        optional($.type_parameters),
        '=',
        optional('|'),
        field('value', $._type),
        ';',
      ),

    enum_declaration: $ =>
      seq(
        'enum',
        field('name', $.identifier),
        optional(seq(':', $._type)),
        '{',
        commaSep($.enum_member),
        optional(','),
        '}',
      ),

    enum_member: $ =>
      seq(
        field('name', $.identifier),
        choice(
          optional(seq('=', $._expression)),
          seq('(', commaSep($._type), optional(','), ')'),
          seq('{', commaSep($.enum_variant_field), optional(','), '}'),
        ),
      ),

    enum_variant_field: $ => seq(field('name', $.identifier), ':', field('type', $._type)),

    if_statement: $ =>
      prec.right(
        seq(
          'if',
          '(',
          field('condition', $._expression),
          ')',
          field('consequence', $._statement),
          optional(seq('else', field('alternative', $._statement))),
        ),
      ),

    for_statement: $ =>
      seq(
        'for',
        '(',
        choice($._for_c_style, $._for_in_of),
        ')',
        field('body', $._statement),
      ),

    _for_c_style: $ =>
      seq(
        field('initializer', optional(choice($._for_declaration, $._expression))),
        ';',
        field('condition', optional($._expression)),
        ';',
        field('update', optional($._expression)),
      ),

    _for_declaration: $ =>
      seq(
        choice('const', 'let'),
        field('name', $.identifier),
        optional(seq(':', field('type', $._type))),
        optional(seq('=', field('value', $._expression))),
      ),

    _for_in_of: $ =>
      seq(
        choice('const', 'let'),
        field('left', choice($.identifier, $.array_pattern, $.object_pattern)),
        choice('of', 'in'),
        field('right', $._expression),
      ),

    while_statement: $ =>
      seq('while', '(', field('condition', $._expression), ')', field('body', $._statement)),

    do_statement: $ =>
      seq(
        'do',
        field('body', $._statement),
        'while',
        '(',
        field('condition', $._expression),
        ')',
        optional(';'),
      ),

    switch_statement: $ =>
      seq('switch', '(', field('value', $._expression), ')', '{', repeat($.switch_case), '}'),

    switch_case: $ =>
      seq(
        choice(seq('case', $._expression), 'default'),
        ':',
        repeat($._statement),
      ),

    match_statement: $ =>
      prec(1, seq('match', '(', field('value', $._expression), ')', '{', repeat($.match_arm), '}')),

    match_expression: $ =>
      prec.right(seq('match', field('value', $._expression), '{', repeat($.match_arm), '}')),

    match_arm: $ =>
      seq(
        field('pattern', choice(seq('(', $._match_pattern, ')'), $._match_pattern)),
        optional(seq('if', '(', field('guard', $._expression), ')')),
        '=>',
        field('body', choice($.block, seq($._expression, optional(choice(',', ';'))))),
      ),

    _match_pattern: $ =>
      choice(
        $.match_call_pattern,
        $.match_path_pattern,
        $.match_range_pattern,
        $.identifier,
        $.number,
        $.string,
        $.true,
        $.false,
        $.null,
        $.none,
        $.match_object_pattern,
        $.match_array_pattern,
      ),

    match_range_pattern: $ =>
      seq(optional($.number), choice('..', '..='), optional($.number)),

    // Destructures a payload: `Ok(value)`, `Err(error)` and the qualified form
    // `Shape.Circle(radius)`. The path is spelled out instead of reusing
    // `member_expression`, to keep pattern position free of expression rules.
    match_call_pattern: $ =>
      seq(
        field('path', seq($.identifier, optional(seq('.', $.identifier)))),
        '(',
        commaSep($._match_pattern),
        optional(','),
        ')',
      ),

    match_path_pattern: $ => seq($.identifier, '.', $.identifier),

    match_object_pattern: $ => seq('{', commaSep($._match_object_field), optional(','), '}'),

    _match_object_field: $ =>
      choice(
        seq(field('key', $.identifier), ':', field('value', $._match_pattern)),
        field('binding', $.identifier),
      ),

    match_array_pattern: $ =>
      seq('[', commaSep(choice($._match_pattern, $.match_rest_pattern)), optional(','), ']'),

    match_rest_pattern: $ => seq('...', field('name', $.identifier)),

    return_statement: $ => prec.right(seq('return', optional($._expression), optional(';'))),
    break_statement: $ => prec.right(seq('break', optional(field('value', $._expression)), optional(';'))),
    continue_statement: _ => seq('continue', optional(';')),
    throw_statement: $ => seq('throw', $._expression, optional(';')),

    try_statement: $ =>
      seq(
        'try',
        field('body', $.block),
        repeat(seq('catch', '(', $.identifier, optional(seq(':', $._type)), ')', $.block)),
        optional(seq('finally', $.block)),
      ),

    block: $ => seq('{', repeat($._statement), '}'),

    expression_statement: $ => seq($._expression, optional($.onfail_clause), optional(';')),

    // ── Types ───────────────────────────────────────────────────────────────
    type_parameters: $ => seq('<', commaSep($.type_parameter), '>'),
    type_parameter: $ =>
      seq(field('name', $.identifier), optional(seq('extends', $._type))),

    _type: $ =>
      choice(
        $.primitive_type,
        $.reference_type,
        $.readonly_type,
        $.generic_type,
        $.array_type,
        $.sized_array_type,
        $.keyof_type,
        $.conditional_type,
        $.tuple_type,
        $.object_type,
        $.function_type,
        $.union_type,
        $.intersection_type,
        $.optional_type,
        $.literal_type,
        $.type_identifier,
        $.parenthesized_type,
      ),

    literal_type: $ => choice($.string, $.number, $.true, $.false, $.null, $.none),

    primitive_type: _ =>
      choice(
        'i32', 'i64', 'i128', 'isize',
        'u32', 'u64', 'u128', 'usize',
        'f32', 'f64', 'f128', 'decimal',
        'boolean', 'string', 'void', 'unknown', 'never',
      ),

    reference_type: $ => prec(2, seq('&', optional('mut'), $._type)),
    readonly_type: $ => prec(2, seq('readonly', $._type)),
    optional_type: $ => prec(3, seq($._type, '?')),
    array_type: $ => prec(4, seq($._type, '[', ']')),
    sized_array_type: $ =>
      seq('[', field('element', $._type), ';', field('length', $._expression), ']'),
    keyof_type: $ => prec.right(3, seq('keyof', $._type)),
    conditional_type: $ =>
      prec.right(
        1,
        seq(
          field('check', $._type),
          'extends',
          field('bound', $._type),
          '?',
          field('consequence', $._type),
          ':',
          field('alternative', $._type),
        ),
      ),
    generic_type: $ => prec(5, seq(field('name', $.type_identifier), $.type_arguments)),
    type_arguments: $ => seq('<', commaSep($._type), '>'),
    tuple_type: $ => seq('[', commaSep($._type), optional(','), ']'),
    parenthesized_type: $ => seq('(', $._type, ')'),

    union_type: $ => prec.left(1, seq($._type, repeat1(seq('|', $._type)))),

    // `&` combines object types, and the compiler accepts it only as the whole
    // body of a `type` alias. It binds looser than `readonly`, so
    // `readonly A & B` groups as `(readonly A) & B` the way the compiler parses
    // it. The associativity has to be right: with `prec.left` the parser closed
    // the alias after the first operand, since the `;` is optional, and read the
    // rest as a `&` borrow statement.
    intersection_type: $ => prec.right(1, seq($._type, repeat1(seq('&', $._type)))),

    function_type: $ =>
      prec.right(
        seq(
          $._function_type_parameters,
          '=>',
          $._type,
          optional(seq('throws', field('throws', $._type))),
        ),
      ),

    _function_type_parameters: $ => seq('(', commaSep($._function_type_parameter), optional(','), ')'),

    _function_type_parameter: $ => seq(optional(seq($.identifier, ':')), $._type),

    object_type: $ =>
      seq('{', repeat(seq(optional(repeat($.decorator)), $.property_signature)), '}'),

    property_signature: $ =>
      seq(
        optional('readonly'),
        field('name', choice($.identifier, 'readonly')),
        optional('?'),
        ':',
        field('type', $._type),
        optional(choice(';', ',')),
      ),

    type_identifier: $ => alias($.identifier, $.type_identifier),

    // ── Parameters ──────────────────────────────────────────────────────────
    parameters: $ => seq('(', commaSep($.parameter), optional(','), ')'),

    parameter: $ =>
      seq(
        optional(repeat($.decorator)),
        optional('...'),
        optional('mut'),
        field('name', choice($.identifier, $.array_pattern, $.object_pattern)),
        optional(alias($._optional_parameter_question, '?')),
        optional(seq(':', field('type', $._type))),
        optional(seq('=', field('default', $._expression))),
      ),

    decorator: $ =>
      seq('@', field('name', $.identifier), optional($.arguments)),

    // ── Expressions ─────────────────────────────────────────────────────────
    _expression: $ =>
      choice(
        $.arrow_function,
        $.binary_expression,
        $.in_expression,
        $.is_expression,
        $.range_expression,
        $.unary_expression,
        $.borrow_expression,
        $.ternary_expression,
        $.assignment_expression,
        $.fallible_as_expression,
        $.as_expression,
        $.await_expression,
        $.comptime_expression,
        $.loop_expression,
        $.match_expression,
        $.unary_postfix_expression,
        $.try_expression,
        $.primary_expression,
      ),

    borrow_expression: $ => prec.right(PREC.unary, seq('&', optional('mut'), $._expression)),

    comptime_expression: $ => seq('comptime', '(', field('value', $._expression), ')'),

    loop_expression: $ => seq('loop', field('body', $.block)),

    primary_expression: $ =>
      choice(
        $.identifier,
        $.number,
        $.string,
        $.template_string,
        $.regex,
        $.true,
        $.false,
        $.null,
        $.none,
        $.array_literal,
        $.repeated_array_literal,
        $.object,
        $.enum_struct_variant_expression,
        $.call_expression,
        $.member_expression,
        $.subscript_expression,
        $.new_expression,
        $.topic,
        $.parenthesized_expression,
      ),

    topic: _ => '%',

    parenthesized_expression: $ => seq('(', $._expression, ')'),

    arrow_function: $ =>
      prec.right(
        choice(
          seq(
            optional('async'),
            field('parameter', $.identifier),
            '=>',
            field('body', choice($.block, $._expression)),
          ),
          seq(
            optional('async'),
            optional($.type_parameters),
            field('parameters', $.parameters),
            optional(seq(':', field('return_type', $._type))),
            optional(seq('throws', field('throws_type', $._type))),
            '=>',
            field('body', choice($.block, $._expression)),
          ),
        ),
      ),

    call_expression: $ =>
      choice(
        prec(
          PREC.call,
          seq(field('function', $.primary_expression), field('arguments', $.arguments)),
        ),
        prec(
          PREC.equality,
          seq(
            field('function', $.primary_expression),
            $.call_type_arguments,
            field('arguments', $.arguments),
          ),
        ),
      ),

    call_type_arguments: $ => seq(token.immediate('<'), commaSep($._type), '>'),

    new_expression: $ =>
      prec.right(
        seq('new', field('constructor', $.identifier), optional($.type_arguments), optional($.arguments)),
      ),

    arguments: $ => seq('(', commaSep(choice($._expression, $.spread_element)), optional(','), ')'),
    spread_element: $ => seq('...', $._expression),

    member_expression: $ =>
      prec(
        PREC.member,
        seq(
          field('object', $.primary_expression),
          choice('.', '?.'),
          field('property', $.identifier),
        ),
      ),

    // Derived from `member_expression` on purpose. Spelling it out as
    // `identifier '.' identifier object` made the parser commit to this rule at
    // the dot and then demand a `{`, so every plain `v.b` in the language was an
    // ERROR node while `v?.b` parsed.
    enum_struct_variant_expression: $ =>
      prec(
        PREC.call,
        seq(field('variant', $.member_expression), field('payload', $.object)),
      ),

    subscript_expression: $ =>
      prec(
        PREC.member,
        seq(field('object', $.primary_expression), '[', field('index', $._expression), ']'),
      ),

    await_expression: $ => prec(PREC.unary, seq('await', $._expression)),

    fallible_as_expression: $ =>
      prec.left(
        PREC.as + 1,
        seq($._expression, 'as', $._type, alias($._fallible_as_question, '?')),
      ),

    as_expression: $ => prec.left(PREC.as, seq($._expression, 'as', choice('const', $._type))),

    unary_expression: $ =>
      prec.right(
        PREC.unary,
        seq(field('operator', choice('!', 'not', '-', '+', '~', 'typeof')), $._expression),
      ),

    unary_postfix_expression: $ =>
      prec.left(PREC.postfix, seq($._expression, field('operator', choice('++', '--', '!')))),

    try_expression: $ => seq($._expression, alias($._try_question, '?')),

    ternary_expression: $ =>
      prec.right(
        PREC.ternary,
        seq($._expression, '?', $._expression, ':', $._expression),
      ),

    assignment_expression: $ =>
      prec.right(
        PREC.assign,
        seq(
          field('left', $._expression),
          field('operator', choice('=', '+=', '-=', '*=', '/=', '%=', '**=', '&=', '|=', '^=', '<<=', '>>=', '>>>=', '&&=', '||=', '??=')),
          field('right', $._expression),
        ),
      ),

    binary_expression: $ => {
      const table = [
        ['|>', PREC.pipeline],
        ['??', PREC.nullish],
        ['||', PREC.or],
        ['or', PREC.or],
        ['&&', PREC.and],
        ['and', PREC.and],
        ['|', PREC.bit_or],
        ['^', PREC.bit_xor],
        ['&', PREC.bit_and],
        ['===', PREC.equality],
        ['!==', PREC.equality],
        ['<', PREC.relational],
        ['>', PREC.relational],
        ['<=', PREC.relational],
        ['>=', PREC.relational],
        ['<<', PREC.shift],
        ['>>', PREC.shift],
        ['>>>', PREC.shift],
        ['+', PREC.additive],
        ['-', PREC.additive],
        ['*', PREC.multiplicative],
        ['/', PREC.multiplicative],
        ['%', PREC.multiplicative],
        ['**', PREC.exponent],
      ];
      return choice(
        ...table.map(([operator, precedence]) =>
          (operator === '**' ? prec.right : prec.left)(
            precedence,
            seq(
              field('left', $._expression),
              field('operator', operator),
              field('right', $._expression),
            ),
          ),
        ),
      );
    },

    // `key in object` / `key not in object`, membership/property-existence
    // check over an array, Range, Map, Set, or (structurally) an object.
    in_expression: $ =>
      prec.left(
        PREC.in,
        seq(
          field('left', $._expression),
          field('operator', choice('in', seq('not', 'in'))),
          field('right', $._expression),
        ),
      ),

    is_expression: $ =>
      prec.left(
        PREC.in,
        seq(
          field('left', $._expression),
          field('operator', choice('is', seq('not', 'is'))),
          field('right', $._type),
        ),
      ),

    // `start..end` (exclusive) / `start..=end` (inclusive), a bounded
    // numeric range.
    range_expression: $ =>
      prec.left(
        PREC.range,
        seq(
          field('left', $._expression),
          field('operator', choice('..', '..=')),
          field('right', $._expression),
        ),
      ),

    array_literal: $ => seq('[', commaSep(choice($._expression, $.spread_element)), optional(','), ']'),
    repeated_array_literal: $ =>
      seq('[', field('value', $._expression), ';', field('length', $._expression), ']'),

    object: $ =>
      seq('{', commaSep(choice($.pair, $.spread_element, $.identifier)), optional(','), '}'),
    pair: $ =>
      seq(field('key', choice($.identifier, $.string, $.number)), ':', field('value', $._expression)),

    // ── Literals ────────────────────────────────────────────────────────────
    identifier: _ => /[a-zA-Z_][a-zA-Z0-9_]*/,

    number: _ => {
      const decimal = /[0-9][0-9_]*/;
      const float = /[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?/;
      const exp = /[0-9][0-9_]*[eE][+-]?[0-9]+/;
      const hex = /0[xX][0-9a-fA-F_]+/;
      const oct = /0[oO][0-7_]+/;
      const bin = /0[bB][01_]+/;
      const typedDecimal = /[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9]+)?[dD]/;
      const typedFloat = /[0-9][0-9_]*(\.[0-9][0-9_]*)?([eE][+-]?[0-9]+)?f(32|64|128)/;
      const typedInteger = /[0-9][0-9_]*(u8|u16|u32|u64|u128|usize|i8|i16|i32|i64|i128|isize)/;
      return token(choice(typedDecimal, typedFloat, typedInteger, float, exp, hex, oct, bin, decimal));
    },

    string: _ =>
      token(
        choice(
          seq(
            '"""',
            repeat(choice(/[^"\\]/, /\\./, seq('"', /[^"\\]/), seq('""', /[^"\\]/))),
            '"""',
          ),
          seq('"', repeat(choice(/[^"\\]/, /\\./)), '"'),
          seq("'", repeat(choice(/[^'\\]/, /\\./)), "'"),
        ),
      ),

    template_string: _ => token(seq('`', repeat(choice(/[^`\\]/, /\\./)), '`')),

    regex: _ => token(seq('/', /[^/\\\n]*(\\.[^/\\\n]*)*/, '/', /[a-z]*/)),

    true: _ => 'true',
    false: _ => 'false',
    null: _ => 'null',
    none: _ => 'None',
  },
});

function commaSep(rule) {
  return optional(seq(rule, repeat(seq(',', rule))));
}
