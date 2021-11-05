type RedisStreamClient = {};

export function connect(
  connectionString: String,
  streamKey: String,
  groupName: string,
  consumerPrefix: string
): RedisStreamClient;
