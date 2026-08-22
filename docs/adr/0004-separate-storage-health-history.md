# Separate Storage Health History

Status: accepted

Storage Health Records are stored and retained separately from the Hardware Archive. Hardware Archive data tracks CPU, memory, GPU, temperature, and process performance history, while Storage Health history tracks long-term device health signals that are useful even when performance history is kept for a shorter period.

This lets the Hardware Dashboard show the latest storage health and recent changes, while future historical views can follow long-term storage health trends without coupling them to the retention period or aggregation shape of utilization history.
