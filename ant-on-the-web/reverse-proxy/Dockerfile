FROM nginx
COPY ./reverse-proxy/data/nginx/app.conf /etc/nginx/nginx.conf
RUN echo $(cat /etc/nginx/nginx.conf)
COPY ./index.html /etc/nginx/html/index.html
