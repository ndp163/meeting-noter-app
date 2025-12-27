#import <Foundation/Foundation.h>

@interface WhisperKitBridge : NSObject

+ (instancetype)shared;

- (void)initializeWithModelPath:(NSString * _Nullable)modelPath 
                     completion:(void (^)(BOOL success, NSString * _Nullable error))completion;

- (void)transcribeWithAudioData:(NSData *)audioData 
                     completion:(void (^)(NSString * _Nullable transcription, NSString * _Nullable error))completion;

- (void)transcribeStreamWithAudioData:(NSData *)audioData 
                           completion:(void (^)(NSString * _Nullable transcription, NSString * _Nullable error))completion;

- (void)shutdown;

@end
