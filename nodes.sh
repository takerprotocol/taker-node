#!/bin/bash

# create logs directory
mkdir logs

# start node
RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/alice \
  --chain testnet_local \
  --port 30333 \
  --rpc-port 9933 \
  --rpc-cors all \
  --node-key 1200000000000000000000000000000000000000000000000000000011111111 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --validator \
  >> ./logs/node_alice.log &

sleep 2
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["babe","","0x0447d34c079f9a8f3d62ae6592e7c5c4e334d25cd18673b81c67cb910314cf65"]}'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["gran","","0xDFCE90621427FF95C38F10BCB2BC4020A0C629BF121F1C748265040640692289"]}'
curl http://localhost:9933 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["imon","","0x0447d34c079f9a8f3d62ae6592e7c5c4e334d25cd18673b81c67cb910314cf65"]}'

RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/bob \
  --chain testnet_local \
  --port 30334 \
  --rpc-port 9934 \
  --rpc-cors all \
  --node-key 0000000000000000000000000000000000000000000000000000000022222222 \
  --validator \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMjGeCC3dLqErxkpRA2KiqDvY1KuLFbYm8pPjywnjTTT9 \
  >> ./logs/node_bob.log &

sleep 2

curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["babe","","0x3e9562351b3ed2e1136b3d5e29263e6e4d03adb2a3611987750680fe283aac14"]}'
curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["gran","","0xC632AAB355782C01C275FFC7A863ED78C709895B4BBD23B53EE31B76C3F659A8"]}'
curl http://localhost:9934 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["imon","","0x3e9562351b3ed2e1136b3d5e29263e6e4d03adb2a3611987750680fe283aac14"]}'

RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/charlie \
  --chain testnet_local \
  --port 30335 \
  --rpc-port 9935 \
  --rpc-cors all \
  --node-key 0000000000000000000000000000000000000000000000000000000033333333 \
  --validator \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMjGeCC3dLqErxkpRA2KiqDvY1KuLFbYm8pPjywnjTTT9 \
  >> ./logs/node_charlie.log &

sleep 2

curl http://localhost:9935 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["babe","","0x142b1fe25707d2b580f4e7491599ebe3a301ea9f875f7d04fa9649373982fa63"]}'
curl http://localhost:9935 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["gran","","0xEDD2ACF928369505B7D52AE2D6F65F22B4B4040F2B993B5F24B9099055A80820"]}'
curl http://localhost:9935 -H "Content-Type:application/json;charset=utf-8" -d '{"jsonrpc":"2.0","id":1,"method":"author_insertKey","params": ["imon","","0x142b1fe25707d2b580f4e7491599ebe3a301ea9f875f7d04fa9649373982fa63"]}'

sleep 4
pkill taker
sleep 4

RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/alice \
  --chain testnet_local \
  --port 30333 \
  --rpc-port 9933 \
  --rpc-cors all \
  --node-key 1200000000000000000000000000000000000000000000000000000011111111 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --validator \
  >> ./logs/node_alice.log &

sleep 2

RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/bob \
  --chain testnet_local \
  --port 30334 \
  --rpc-port 9934 \
  --rpc-cors all \
  --node-key 0000000000000000000000000000000000000000000000000000000022222222 \
  --validator \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMjGeCC3dLqErxkpRA2KiqDvY1KuLFbYm8pPjywnjTTT9 \
  >> ./logs/node_bob.log &

sleep 2

RUST_LOG=info nohup ./target/release/taker-node \
  --base-path ./base/charlie \
  --chain testnet_local \
  --port 30335 \
  --rpc-port 9935 \
  --rpc-cors all \
  --node-key 0000000000000000000000000000000000000000000000000000000033333333 \
  --validator \
  --enable-offchain-indexing true \
  --blocks-pruning=archive \
  --state-pruning=archive \
  --ethapi="debug,trace,txpool" \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWMjGeCC3dLqErxkpRA2KiqDvY1KuLFbYm8pPjywnjTTT9 \
  >> ./logs/node_charlie.log &
