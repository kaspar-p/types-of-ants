FROM alpine

RUN apk add curl bash openssl

RUN mkdir -p /getssl/dns_scripts

RUN curl --silent https://raw.githubusercontent.com/srvrco/getssl/latest/getssl \
  --output /getssl/getssl && chmod 700 /getssl/getssl
COPY ./dns_scripts/dns_add_cloudflare /getssl/dns_scripts/dns_add_cloudflare
COPY ./dns_scripts/dns_del_cloudflare /getssl/dns_scripts/dns_del_cloudflare

ARG ANT_RENEWING_CERTIFICATES_FQDN
ENV ANT_RENEWING_CERTIFICATES_FQDN ${ANT_RENEWING_CERTIFICATES_FQDN}

RUN mkdir /getssl/${ANT_RENEWING_CERTIFICATES_FQDN}
COPY ./getssl.cfg /getssl/getssl.cfg
COPY ./${ANT_RENEWING_CERTIFICATES_FQDN}/getssl.cfg /getssl/${ANT_RENEWING_CERTIFICATES_FQDN}/getssl.cfg

COPY ./build/release/cronjobs /etc/crontabs/root
CMD ["crond", "-f", "-d", "8"]

# CMD /getssl/getssl -q -f -w /getssl ${ANT_RENEWING_CERTIFICATES_FQDN}
