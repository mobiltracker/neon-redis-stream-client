"use strict";
const streamFFI = require("./index.node");
class RedisStreamClient {
    constructor(connectionString, streamKey, groupName, consumerPrefix) {
        this.connectionString = connectionString;
        this.streamKey = streamKey;
        this.groupName = groupName;
        this.consumerPrefix = consumerPrefix;
        this.clientRef = null;
    }
    connect() {
        // //"redis://127.0.0.1",
        // "test-stream",
        // "test-group",
        // "test-consumer"
        const client = streamFFI.connect(this.connectionString, this.streamKey, this.groupName, this.consumerPrefix);
        this.clientRef = client;
    }
    async readNext() {
        const msg = await streamFFI.readNext(this.clientRef);
        return msg;
    }
    async ackMessageId(msgId) {
        return await streamFFI.ackMessage(this.clientRef, msgId);
    }
}
module.exports = { RedisStreamClient };
