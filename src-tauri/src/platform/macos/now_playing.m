#import <Foundation/Foundation.h>
#import <dispatch/dispatch.h>

extern void MRMediaRemoteGetNowPlayingInfo(dispatch_queue_t queue, void (^handler)(NSDictionary *info));

bool macos_has_now_playing_session(void) {
    __block BOOL hasSession = NO;
    dispatch_semaphore_t sem = dispatch_semaphore_create(0);

    MRMediaRemoteGetNowPlayingInfo(dispatch_get_global_queue(DISPATCH_QUEUE_PRIORITY_DEFAULT, 0), ^(NSDictionary *info) {
        hasSession = info != nil && info.count > 0;
        dispatch_semaphore_signal(sem);
    });

    dispatch_semaphore_wait(sem, dispatch_time(DISPATCH_TIME_NOW, 1 * NSEC_PER_SEC));
    return hasSession;
}
