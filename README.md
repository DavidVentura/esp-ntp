
This project is a from-scratch, stratum-1 NTP server, which gets its time from a GPS receiver (u-blox NEO).

It runs on an ESP32, and can be run on PC to test/debug.


There's no consideration for the initial leap seconds stored on the GPS receiver, that means that for the first 12.5 minutes of usage, the leap seconds will be off -- in my case, by 3 seconds.
After the initial sync, the leap seconds will be stored in the receiver's RTC memory.
