FROM alpine
ARG TARGETARCH

COPY /assets/ /css/assets/
COPY /templates/ /css/templates/
COPY /${TARGETARCH}-executables/css /usr/bin/

WORKDIR "/css/"
ENTRYPOINT "/usr/bin/css"