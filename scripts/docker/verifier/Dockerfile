ARG PYTHON_IMAGE=python:3.12-slim
FROM ${PYTHON_IMAGE}

ARG PRE_COMMIT_VERSION=4.0.1

RUN apt-get update -qq \
    && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq --no-install-recommends \
      bash \
      ca-certificates \
      coreutils \
      docker-cli \
      findutils \
      git \
      grep \
      sed \
      util-linux \
    && rm -rf /var/lib/apt/lists/* \
    && python -m pip install --no-cache-dir "pre-commit==${PRE_COMMIT_VERSION}"

ENTRYPOINT []
CMD ["pre-commit", "--version"]
