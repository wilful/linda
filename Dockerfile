FROM ubuntu

ENV RUST_BACKTRACE=1
ENV RUST_LOG=garage=info

COPY target/release/linda /
CMD [ "/linda", "exec"]

