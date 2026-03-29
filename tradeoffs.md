not going to use snapshot for any major decisions like evictions etc thus could be a near estimate 
so could be different point in times thus could lead to some fields being in stale state than other on same snapshot 
could be more accurate using mutex on stats for true concurency but that would lead to less performat info usage which is pretty ofently logged for dashboards