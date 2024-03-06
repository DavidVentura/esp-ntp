
This project is a from-scratch, stratum-1 NTP server, which gets its time from a GPS receiver (u-blox NEO).

It runs on an ESP32, and can be run on PC to test/debug.


There's no consideration for the initial leap seconds stored on the GPS receiver, that means that for the first 12.5 minutes of usage, the leap seconds will be off -- in my case, by 3 seconds. This will depend on how old your receiver's firmware is; 3 seconds puts my firmware [before 2012](https://en.wikipedia.org/wiki/Leap_second).

After the initial sync, the leap seconds will be stored in the receiver's RTC memory.

This project runs:
- An UBX parser on the serial data in the GPS
- A stratum-1 NTP server, on port 123 UDP.
- An HTTP server for prometheus metrics, port 80, /metrics.

The metrics currently look like
```
gps_clock_accuracy_us{quantile="0.10"} 11199
gps_clock_accuracy_us{quantile="0.50"} 11204
gps_clock_accuracy_us{quantile="0.90"} 11210
gps_clock_accuracy_us{quantile="0.99"} 11211
rtc_clock_adjust_ms{quantile="0.10"} -159
rtc_clock_adjust_ms{quantile="0.50"} 0
rtc_clock_adjust_ms{quantile="0.90"} 159
rtc_clock_adjust_ms{quantile="0.99"} 190
has_fix 0
received_ntp_queries 10
answered_ntp_queries 10
```

and I'm planning to add the # of visible satellites
