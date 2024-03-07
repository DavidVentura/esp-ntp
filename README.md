
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
satellite_count{quantile="0.10"} 1
satellite_count{quantile="0.50"} 1
satellite_count{quantile="0.90"} 2
satellite_count{quantile="0.99"} 2
gps_clock_accuracy_ns{quantile="0.10"} 48
gps_clock_accuracy_ns{quantile="0.50"} 76
gps_clock_accuracy_ns{quantile="0.90"} 92
gps_clock_accuracy_ns{quantile="0.99"} 440
rtc_clock_adjust_ms{quantile="0.10"} -9
rtc_clock_adjust_ms{quantile="0.50"} 0
rtc_clock_adjust_ms{quantile="0.90"} 4
rtc_clock_adjust_ms{quantile="0.99"} 148
sensor_uptime_sec 70
has_fix 1
received_ntp_queries 26
answered_ntp_queries 26
```
