FROM rust:1.25.0

WORKDIR /var/disease/
COPY . .

RUN rustc --version
RUN cargo install

CMD ["disease2018"]
