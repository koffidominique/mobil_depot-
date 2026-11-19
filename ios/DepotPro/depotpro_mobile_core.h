#ifndef DEPOTPRO_MOBILE_CORE_H
#define DEPOTPRO_MOBILE_CORE_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Exécute une commande JSON : {"command":"...", "payload":{...}}
 * Réponse : {"ok":true,"data":{...}} ou {"ok":false,"error":{"code":"...","message":"..."}}
 * Le pointeur retourné doit être libéré avec depot_free.
 */
char *depot_execute(const char *input);
void depot_free(char *ptr);
const char *depot_version(void);

#ifdef __cplusplus
}
#endif

#endif /* DEPOTPRO_MOBILE_CORE_H */
