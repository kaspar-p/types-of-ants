FROM nginx

ARG FQDN
ARG DESTINATION_ANT_WORKER_NUM
ARG WEBSERVER_PORT
ARG SSL_CERT_PATH
ARG SSL_KEY_PATH

# Copy the templates nginx.conf into the container
COPY ./data/nginx.conf.mo /tmp/ant-on-the-web/nginx.conf.mo
RUN curl -sSL https://raw.githubusercontent.com/tests-always-included/mo/master/mo -o mo
RUN chmod +x ./mo
RUN ./mo /tmp/ant-on-the-web/nginx.conf.mo > /etc/nginx/nginx.conf

# Copy the public and private keys into the container
COPY "$SSL_CERT_PATH" "/ant-on-the-web/${FQDN}/cert.pem"
COPY "$SSL_KEY_PATH" "/ant-on-the-web/${FQDN}/key.pem"
