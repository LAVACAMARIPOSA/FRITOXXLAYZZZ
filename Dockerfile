FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY bot /app/bot
RUN chmod +x /app/bot && mkdir -p /app/data
WORKDIR /app
EXPOSE 7860
CMD ["./bot"]
