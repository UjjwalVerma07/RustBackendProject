#ifndef STRINGUTILS_WRAPPER_H
#define STRINGUTILS_WRAPPER_H

#include <stddef.h> /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

/*
 * String-utilities C interface.
 *
 * All functions take a NUL-terminated C string (`const char *input`) and
 * operate on the bytes preceding the terminating NUL. Uppercase conversion and
 * vowel counting are ASCII-defined: any byte whose value is greater than 127 is
 * left unchanged and is never counted as a vowel.
 *
 * Memory-ownership model:
 *   - str_reverse and to_uppercase return a newly heap-allocated,
 *     NUL-terminated C string. Ownership transfers to the caller, who MUST
 *     release it exactly once via free_string.
 *   - On allocation failure these functions return NULL and leave the input
 *     unmodified.
 *   - str_length and count_vowels return a value and allocate nothing.
 */

/*
 * Returns the length of `input` in bytes, excluding the terminating NUL.
 * Returns 0 for an empty string.
 */
size_t str_length(const char *input);

/*
 * Returns the count of ASCII vowels (a, e, i, o, u; case-insensitive) among the
 * bytes of `input` preceding the terminating NUL. Bytes greater than 127 are
 * never counted.
 */
size_t count_vowels(const char *input);

/*
 * Returns a newly heap-allocated, NUL-terminated copy of `input` with its bytes
 * (those preceding the terminating NUL) in reverse byte order. For an empty
 * input, returns a newly allocated one-byte buffer containing only the
 * terminating NUL. Returns NULL on allocation failure. The caller owns the
 * returned pointer and must release it with free_string.
 */
char *str_reverse(const char *input);

/*
 * Returns a newly heap-allocated, NUL-terminated copy of `input` in which ASCII
 * lowercase letters are folded to uppercase; bytes greater than 127 pass
 * through unchanged. For an empty input, returns a newly allocated one-byte
 * buffer containing only the terminating NUL. Returns NULL on allocation
 * failure. The caller owns the returned pointer and must release it with
 * free_string.
 */
char *to_uppercase(const char *input);

/*
 * Releases memory previously returned by str_reverse or to_uppercase, using the
 * allocator matching the one used to allocate it. Passing NULL is a no-op.
 */
void free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* STRINGUTILS_WRAPPER_H */
