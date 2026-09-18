#include "wrapper.h"

#include <cstddef> // std::size_t
#include <cstring> // std::strlen, std::memcpy
#include <new>     // std::nothrow

// All definitions carry C linkage matching the declarations in wrapper.h so
// that symbol names are not mangled and the interface is FFI-compatible. No
// exceptions cross the C boundary: allocations use `new (std::nothrow)` and
// yield nullptr on failure rather than throwing std::bad_alloc.
extern "C" {

size_t str_length(const char *input) {
    // Byte count before the terminating NUL, excluding the NUL. Empty -> 0.
    return std::strlen(input);
}

size_t count_vowels(const char *input) {
    size_t count = 0;
    for (const char *p = input; *p != '\0'; ++p) {
        // Interpret as unsigned so bytes > 127 are handled correctly and never
        // counted as vowels (ASCII-only semantics).
        unsigned char c = static_cast<unsigned char>(*p);
        switch (c) {
        case 'a':
        case 'A':
        case 'e':
        case 'E':
        case 'i':
        case 'I':
        case 'o':
        case 'O':
        case 'u':
        case 'U':
            ++count;
            break;
        default:
            break;
        }
    }
    return count;
}

char *str_reverse(const char *input) {
    size_t len = std::strlen(input);

    // Allocate len + 1 for the terminating NUL. nothrow -> nullptr on failure.
    char *out = new (std::nothrow) char[len + 1];
    if (out == nullptr) {
        // Allocation failed: leave the input unmodified and signal failure.
        return nullptr;
    }

    for (size_t i = 0; i < len; ++i) {
        out[i] = input[len - 1 - i];
    }
    out[len] = '\0';
    return out;
}

char *to_uppercase(const char *input) {
    size_t len = std::strlen(input);

    char *out = new (std::nothrow) char[len + 1];
    if (out == nullptr) {
        return nullptr;
    }

    for (size_t i = 0; i < len; ++i) {
        unsigned char c = static_cast<unsigned char>(input[i]);
        // ASCII lowercase letters fold to uppercase; bytes > 127 (and every
        // other byte) pass through unchanged.
        if (c >= 'a' && c <= 'z') {
            c = static_cast<unsigned char>(c - ('a' - 'A'));
        }
        out[i] = static_cast<char>(c);
    }
    out[len] = '\0';
    return out;
}

void free_string(char *s) {
    // Matches the `new[]` used by str_reverse / to_uppercase. No-op on nullptr.
    if (s) {
        delete[] s;
    }
}

} // extern "C"
