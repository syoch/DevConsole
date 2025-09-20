target/debug/devconsole_cli send SerialMonitor '{"OpenVPort":{"path":"/dev/ttyACM0","channel_name":"MCU"}}'
target/debug/devconsole_cli send PktUART '{"src":3,"dst_ch_name":"MCU-PktUART"}'
target/debug/devconsole_cli listen MCU-PktUART

target/debug/devconsole_cli send -b MCU-PktUART "\nabc"


KV: [id, data...] (data: [] ==> Request, otherwise ==> Response)
SSP: [svc, data...]

55aa5a 5c0000000000000005b7a9305c305c30