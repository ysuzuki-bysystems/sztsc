#include <freerdp/freerdp.h>

extern BOOL get_access_token_aad(freerdp* instance, char** token, const char* scope, const char* req_cnf);

BOOL get_access_token(freerdp* instance, AccessTokenType tokenType, char** token, size_t count, ...) {
    switch (tokenType) {
        case ACCESS_TOKEN_TYPE_AAD:
            if (count < 2) {
                return FALSE;
            }

            va_list ap = WINPR_C_ARRAY_INIT;
            va_start(ap, count);
            const char* scope = va_arg(ap, const char*);
            const char* req_cnf = va_arg(ap, const char*);
            int rc = get_access_token_aad(instance, token, scope, req_cnf);
            va_end(ap);
            return rc;

        case ACCESS_TOKEN_TYPE_AVD:
        default:
            return FALSE;
    }
}
