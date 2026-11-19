#import <React/RCTBridgeModule.h>
#import "depotpro_mobile_core.h"

/**
 * Module iOS exposé sous le nom "DepotCore". Toutes les règles métier sont dans Rust ;
 * ce fichier ne fait que transcoder la requête JSON et résoudre la promesse JS.
 * L'appel Rust est exécuté sur une file série dédiée : le thread UI n'est jamais bloqué.
 */
@interface DepotCore : NSObject <RCTBridgeModule>
@end

@implementation DepotCore

RCT_EXPORT_MODULE(DepotCore)

static dispatch_queue_t DepotCoreSerialQueue(void) {
  static dispatch_queue_t queue;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    queue = dispatch_queue_create("com.depotpro.mobile.rust-core", DISPATCH_QUEUE_SERIAL);
  });
  return queue;
}

RCT_EXPORT_METHOD(executeCommand:(NSString *)request
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject) {
  if (![request isKindOfClass:[NSString class]]) {
    reject(@"INVALID_REQUEST", @"Requête JSON absente.", nil);
    return;
  }

  NSString *capturedRequest = [request copy];
  dispatch_async(DepotCoreSerialQueue(), ^{
    char *raw = depot_execute([capturedRequest UTF8String]);
    NSString *response = raw ? [NSString stringWithUTF8String:raw] : nil;
    if (raw) {
      depot_free(raw);
    }
    dispatch_async(dispatch_get_main_queue(), ^{
      if (!response) {
        reject(@"RUST_CORE_ERROR", @"Réponse du moteur Rust illisible.", nil);
        return;
      }
      resolve(response);
    });
  });
}

@end
