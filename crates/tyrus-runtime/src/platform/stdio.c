#include <stdarg.h>
#include <stdio.h>

size_t tyrus_stdio_write(const void *buffer, size_t bytes, int descriptor) {
    FILE *stream = descriptor == 2 ? stderr : stdout;
    return fwrite(buffer, 1, bytes, stream);
}

int tyrus_stdio_flush(int descriptor) {
    FILE *stream = descriptor == 2 ? stderr : stdout;
    return fflush(stream);
}

int tyrus_dprintf(int descriptor, const char *format, ...) {
    FILE *stream = descriptor == 2 ? stderr : stdout;
    va_list arguments;
    va_start(arguments, format);
    int result = vfprintf(stream, format, arguments);
    va_end(arguments);
    fflush(stream);
    return result;
}
