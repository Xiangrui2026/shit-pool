FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        fonts-texgyre texlive-xetex texlive-lang-chinese texlive-fonts-recommended \
        texlive-fonts-extra texlive-science texlive-pictures texlive-plain-generic unzip \
    && rm -rf /var/lib/apt/lists/*

COPY compile.sh /usr/local/bin/compile-paper
RUN sed -i 's/\r$//' /usr/local/bin/compile-paper && chmod 0555 /usr/local/bin/compile-paper \
    && mkdir /work

ENV HOME=/tmp TEXMFVAR=/tmp/texmf-var TEXMFCONFIG=/tmp/texmf-config \
    openin_any=p openout_any=p
USER 65534:65534
WORKDIR /work
ENTRYPOINT ["timeout", "--kill-after=5s", "80s", "/usr/local/bin/compile-paper"]
