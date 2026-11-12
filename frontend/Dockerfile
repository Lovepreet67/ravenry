FROM node:22-alpine AS builder

WORKDIR /app

COPY package*.json ./
RUN npm ci

COPY . .

ARG VITE_IAM_API_BASE_URL
ARG VITE_HUGINN_API_BASE_URL

ENV VITE_IAM_API_BASE_URL=${VITE_IAM_API_BASE_URL}
ENV VITE_HUGINN_API_BASE_URL=${VITE_HUGINN_API_BASE_URL}

RUN npm run build

FROM nginx:alpine AS runner

RUN rm -rf /usr/share/nginx/html/*

COPY --from=builder /app/dist /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]