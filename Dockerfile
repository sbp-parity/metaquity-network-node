FROM phusion/baseimage:jammy-1.0.1

# metadata
ARG BUILD_DATE

LABEL xyz.metaquity.image.authors="hello@metaquity.xyz" \
	xyz.metaquity.image.vendor="Metaquity Limited" \
	xyz.metaquity.image.title="Metaquity-Network/metaquity" \
	xyz.metaquity.image.created="${BUILD_DATE}"

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
COPY --chown=metaquity:metaquity --chmod=774 metaquity /usr/bin/metaquity

# check if executable works in this container
RUN /usr/bin/metaquity --version

EXPOSE 9930 9333 9944 30333 30334

CMD ["/usr/bin/metaquity"]
