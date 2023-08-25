FROM phusion/baseimage:jammy-1.0.1

# metadata
ARG VCS_REF
ARG BUILD_DATE

LABEL xyz.metaquity.image.authors="hello@metaquity.xyz" \
	xyz.metaquity.image.vendor="Metaquity Limited" \
	xyz.metaquity.image.title="Metaquity-Network/metaquity-network-node" \
	xyz.metaquity.image.source="https://github.com/Metaquity-Network/metaquity-network-node/blob/${VCS_REF}/Dockerfile" \
	xyz.metaquity.image.revision="${VCS_REF}" \
	xyz.metaquity.image.created="${BUILD_DATE}" \
	xyz.metaquity.image.documentation="https://github.com/Metaquity-Network/metaquity-network-node"

# show backtraces
ENV RUST_BACKTRACE 1

RUN apt-get update && \
	DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
		ca-certificates && \
# apt cleanup
	apt-get autoremove -y && \
	apt-get clean && \
	find /var/lib/apt/lists/ -type f -not -name lock -delete; \
# add user and link ~/.local/share/metaquity to /data
	useradd -m -u 1000 -U -s /bin/sh -d /metaquity metaquity && \
	mkdir -p /data /metaquity/.local/share && \
	chown -R metaquity:metaquity /data && \
	ln -s /data /metaquity/.local/share/metaquity

USER metaquity

# copy the compiled binary to the container
COPY --chown=metaquity:metaquity --chmod=774 metaquity-network /usr/bin/metaquity-network

# check if executable works in this container
RUN /usr/bin/metaquity-network --version

EXPOSE 9930 9333 9944 30333 30334

CMD ["/usr/bin/metaquity-network"]
