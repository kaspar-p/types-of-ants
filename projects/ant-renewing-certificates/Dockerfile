FROM alpine

# Native getssl cloudflare token variable name 
ARG CF_API_TOKEN
ARG TYPESOFANTS_FQDN=beta.typesofants.org
ENV TYPESOFANTS_FQDN ${TYPESOFANTS_FQDN}

RUN apk add curl bash openssl

RUN mkdir /getssl

RUN curl --silent https://raw.githubusercontent.com/srvrco/getssl/latest/getssl \
  --output /getssl/getssl && chmod 700 /getssl/getssl

RUN mkdir /getssl/dns_scripts

RUN curl --silent https://raw.githubusercontent.com/srvrco/getssl/latest/dns_scripts/dns_add_cloudflare \
  --output /getssl/dns_scripts/dns_add_cloudflare && chmod 700 /getssl/dns_scripts/dns_add_cloudflare

RUN curl --silent https://raw.githubusercontent.com/srvrco/getssl/latest/dns_scripts/dns_del_cloudflare \
  --output /getssl/dns_scripts/dns_del_cloudflare && chmod 700 /getssl/dns_scripts/dns_del_cloudflare

RUN mkdir /getssl/${TYPESOFANTS_FQDN}
COPY ./getssl.cfg /getssl/getssl.cfg
COPY ./${TYPESOFANTS_FQDN}/getssl.cfg /getssl/${TYPESOFANTS_FQDN}/getssl.cfg

COPY cronjobs /etc/crontabs/root
# CMD ["crond", "-f", "-d", "8"]

CMD /getssl/getssl -q -f -w /getssl ${TYPESOFANTS_FQDN}
