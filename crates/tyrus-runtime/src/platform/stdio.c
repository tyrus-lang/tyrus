#include <stdarg.h>
#include <stdio.h>

#ifdef _WIN32
#include <fcntl.h>
#include <io.h>
#include <windows.h>

static INIT_ONCE stdio_binary_mode_once = INIT_ONCE_STATIC_INIT;

static BOOL CALLBACK set_standard_streams_to_binary(
    PINIT_ONCE once,
    PVOID parameter,
    PVOID *context
) {
    (void)once;
    (void)parameter;
    (void)context;
    _setmode(_fileno(stdout), _O_BINARY);
    _setmode(_fileno(stderr), _O_BINARY);
    return TRUE;
}

static void ensure_standard_streams_are_binary(void) {
    InitOnceExecuteOnce(
        &stdio_binary_mode_once,
        set_standard_streams_to_binary,
        NULL,
        NULL
    );
}
#else
static void ensure_standard_streams_are_binary(void) {}
#endif

size_t tyrus_stdio_write(const void *buffer, size_t bytes, int descriptor) {
    ensure_standard_streams_are_binary();
    FILE *stream = descriptor == 2 ? stderr : stdout;
    return fwrite(buffer, 1, bytes, stream);
}

int tyrus_stdio_flush(int descriptor) {
    FILE *stream = descriptor == 2 ? stderr : stdout;
    return fflush(stream);
}

int tyrus_dprintf(int descriptor, const char *format, ...) {
    ensure_standard_streams_are_binary();
    FILE *stream = descriptor == 2 ? stderr : stdout;
    va_list arguments;
    va_start(arguments, format);
    int result = vfprintf(stream, format, arguments);
    va_end(arguments);
    fflush(stream);
    return result;
}
