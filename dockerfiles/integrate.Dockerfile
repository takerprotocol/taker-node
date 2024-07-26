
# IMAGE TAKER-NODE
FROM ubuntu:20.04 as taker-node
LABEL maintainer "Taker Team"

RUN apt-get update && \
	apt-get install -y --no-install-recommends curl dstat

COPY  ./taker-node /usr/local/bin
COPY  ./specs/taker-testnet.json /specs/taker-testnet.json
COPY  ./specs/taker-mainnet.json /specs/taker-mainnet.json

RUN	useradd -m -u 1000 -U -s /bin/sh -d /taker taker && \
	mkdir -p /taker/.local/share/taker-node && \
	chown -R taker:taker /taker/.local && \
	ln -s /taker/.local/share/taker-node /data

USER taker
EXPOSE 30333 9933 9944
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/taker-node"]


