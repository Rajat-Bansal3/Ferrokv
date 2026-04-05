not going to use snapshot for any major decisions like evictions etc thus could be a near estimate 
so could be different point in times thus could lead to some fields being in stale state than other on same snapshot 
could be more accurate using mutex on stats for true concurency but that would lead to less performat info usage which is pretty ofently logged for dashboards

using multi threading model instead of redis like single threading model for getting true parellelism instead of single client throughput 

each shard has own rw lock so multiple readers and writer can work simun. thus having adv in high concurrent env  whereas redis dont have any lock overhead at all due to its single thread arch thus way better in low concurrent env

choosing rwlock rather than mutex for read heavy data as concurrent readers and single writer and mutexes for write heavy or read write dominating data

cache line alligning shards 64 for reducing cache line poising or dirty cache hits

moriss counter for snapshots 

timer wheel using ring buf instead of btree

used RESP3 message format so it could be connect from a redis cli if configured to do so 
