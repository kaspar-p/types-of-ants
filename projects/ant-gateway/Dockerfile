FROM nginx

ARG FQDN
ARG ANT_WORKER_NUM_ONE
ARG ANT_WORKER_NUM_TWO
ARG ANT_WORKER_NUM_THREE
ARG WEBSERVER_PORT

# Copy the templates nginx.conf into the container
COPY ./data/nginx.conf.mo /tmp/ant-on-the-web/nginx.conf.mo
RUN curl -sSL https://raw.githubusercontent.com/tests-always-included/mo/master/mo -o mo
RUN chmod +x ./mo
RUN ./mo /tmp/ant-on-the-web/nginx.conf.mo > /etc/nginx/nginx.conf
