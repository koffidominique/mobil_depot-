#import <React/RCTBridgeModule.h>
#import "depotpro_mobile_core.h"

/**
 * Module iOS exposé sous le nom "DepotCore". Toutes les règles métier sont dans Rust ;
 * ce fichier ne fait que transcoder la requête JSON et résoudre la promesse JS.
 */
@interface DepotCore : NSObject <RCTBridgeModule>
@end

@implementation DepotCore

RCT_EXPORT_MODULE(DepotCore)

RCT_EXPORT_METHOD(executeCommand:(NSString *)request
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject) {
  if (![request isKindOfClass:[NSString class]]) {
    reject(@"INVALID_REQUEST", @"Requête JSON absente.", nil);
    return;
  }

  char *raw = depot_execute([request UTF8String]);
  NSString *response = raw ? [NSString stringWithUTF8String:raw] : nil;
  if (raw) {
    depot_free(raw);
  }

  if (!response) {
    reject(@"RUST_CORE_ERROR", @"Réponse du moteur Rust illisible.", nil);
    return;
  }
  resolve(response);
}

@end
