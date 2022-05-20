FROM rustlang/rust:nightly

ADD . /my-source

RUN    cd /my-source \
    && cargo rustc --verbose --release \
    && mv /my-source/target/release/pw-sentiment-server /pw-sentiment-server \
    && rm -rfv /my-source

CMD ["/pw-sentiment-server"]
