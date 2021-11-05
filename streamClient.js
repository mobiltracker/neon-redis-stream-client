"use strict";
import { connect, readNext, ackMessage } from "./index.node";
class RedisStreamClient {
    #clientRef;
    constructor(connectionString, streamKey, groupName, consumerPrefix) {
      this.connectionString = connectionString;
      this.streamKey = streamKey;
      this.groupName = groupName;
      this.consumerPrefix = consumerPrefix;
      this.#clientRef = null;
    }

    connect(){

      // //"redis://127.0.0.1",
      // "test-stream",
      // "test-group",
      // "test-consumer"
      const client = connect(
        this.connectionString,
        this.streamKey,
        this.groupName,
        this.consumerPrefix
      );

      this.#clientRef = client;
    }

    async readNext(){
      const msg = await readNext(this.#clientRef);
      return msg;
    }

    async ackMessage(msgId){
      return await ackMessage(this.#clientRef, msgId);
    }
}

export default {connect, readNext, ackMessage};


