FROM rustlang/rust:nightly

WORKDIR /my-source

ADD . /my-source

RUN cargo rustc --verbose --release

CMD ["/usr/local/cargo/bin/cargo", "run", "--release"]
