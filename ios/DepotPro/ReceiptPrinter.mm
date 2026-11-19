#import <React/RCTBridgeModule.h>
#import <UIKit/UIKit.h>

/** Impression native du PDF : le moteur Rust fournit les données, pas la mise en page. */
@interface ReceiptPrinter : NSObject <RCTBridgeModule>
@end

@implementation ReceiptPrinter

RCT_EXPORT_MODULE(ReceiptPrinter)

RCT_EXPORT_METHOD(printPdf:(NSString *)filePath
                  resolver:(RCTPromiseResolveBlock)resolve
                  rejecter:(RCTPromiseRejectBlock)reject) {
  NSURL *url = [NSURL fileURLWithPath:filePath];
  NSData *data = [NSData dataWithContentsOfURL:url];
  if (!data) {
    reject(@"PDF_NOT_FOUND", @"Fichier PDF introuvable.", nil);
    return;
  }
  UIPrintInteractionController *controller = [UIPrintInteractionController sharedPrintController];
  controller.printingItem = data;
  [controller presentAnimated:YES completionHandler:^(UIPrintInteractionController * _Nonnull printer,
                                                      BOOL completed, NSError * _Nullable error) {
    if (error) {
      reject(@"PRINT_FAILED", error.localizedDescription, error);
    } else {
      resolve(@(completed));
    }
  }];
}

@end
