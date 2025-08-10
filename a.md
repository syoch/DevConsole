```mermaid
%%{init: {'theme': 'dark'}}%%
graph LR
  root[" "]

  subgraph Routing Table
    route_52["52"]
    route_ma["main"]
    route_80["80 (proton)"]
  end
  root --tcp:30000-31000--> route_80
  root --> route_ma
  root --> route_52

  subgraph Interfaces
    if_pr["wg-proton"]
    if_ts["tailscale0"]
    if_w0["wlan0"]
    if_do["docker0"]
    if_b0["br-0"]
    if_e0["eth0"]
  end
  route_80 --> if_pr
  route_ma --192.200.0.0/24--> if_pr
  route_ma --172.17.0.0/16 as 172.17.0.1--> if_do
  route_ma --172.18.0.0/16 as 172.18.0.1--> if_b0
  route_ma --192.168.0.0/24 as 192.168.0.4--> if_e0
  route_ma --10.200.0.0/21 as 10.200.5.176--> if_w0
  route_ma --as 10.200.5.176 hop --> if_w0 --> 10.200.7.254
  route_52 -- (tailscale devices) --> if_ts
```

##
```
# 連絡
- 全体連絡

# 一般
- 一般
- フリーディスカッション
- 制御班
- 回路班
- 設計班

# 講習

# 役職
- インフラ
- 3 役
```

mailto:nitnc-robo-club@googlegroups.com

## robo-rpi 追加設定
```
sudo ip route add default dev wg-proton table 80
sudo ip rule add ipproto tcp dport 30000-31000 lookup 80
```
