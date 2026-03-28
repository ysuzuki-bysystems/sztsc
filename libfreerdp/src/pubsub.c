#include <freerdp/freerdp.h>

int subscribe_channel_connected(wPubSub* p, pChannelConnectedEventHandler h) {
    return PubSub_SubscribeChannelConnected(p, h);
}

int subscribe_channel_disconnected(wPubSub* p, pChannelDisconnectedEventHandler h) {
    return PubSub_SubscribeChannelDisconnected(p, h);
}
