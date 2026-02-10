FROM docker.io/library/rust:1.89-bookworm

# Install build tools and tectonic C/C++ dependencies (with static .a libs)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    git \
    meson \
    ninja-build \
    pkg-config \
    libfontconfig1-dev \
    libfreetype-dev \
    libicu-dev \
    libpng-dev \
    libssl-dev \
    zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*

# Build graphite2 as a static library (Debian -dev package lacks .a)
RUN git clone --depth 1 --branch 1.3.14 https://github.com/silnrsi/graphite.git /tmp/graphite2 && \
    cd /tmp/graphite2 && mkdir build && cd build && \
    cmake .. -DCMAKE_INSTALL_PREFIX=/usr \
             -DCMAKE_INSTALL_LIBDIR=lib/x86_64-linux-gnu \
             -DBUILD_SHARED_LIBS=OFF \
             -DCMAKE_POSITION_INDEPENDENT_CODE=ON && \
    make -j$(nproc) && make install && \
    rm -rf /tmp/graphite2

# Build harfbuzz as a static library (Debian -dev package lacks .a)
RUN git clone --depth 1 --branch 6.0.0 https://github.com/harfbuzz/harfbuzz.git /tmp/harfbuzz && \
    cd /tmp/harfbuzz && \
    meson setup build --default-library=static --prefix=/usr \
      -Dgraphite2=enabled -Dicu=enabled -Dfreetype=enabled \
      -Dglib=disabled -Dgobject=disabled -Dcairo=disabled \
      -Dtests=disabled -Ddocs=disabled -Dbenchmark=disabled && \
    cd build && ninja -j$(nproc) && ninja install && \
    rm -rf /tmp/harfbuzz

# Enable semi-static linking: tectonic will prefer .a over .so when available
ENV TECTONIC_PKGCONFIG_FORCE_SEMI_STATIC=1

WORKDIR /usr/local/src

ENTRYPOINT ["cargo"]
