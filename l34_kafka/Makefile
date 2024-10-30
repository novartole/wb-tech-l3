all:
	docker compose up -d

pg:
	docker compose up -d postgres

kafka:
	docker compose up -d kafka

app:
	docker compose up -d web-app

down-v:
	docker compose down -v

watch-kafka:
	docker exec -it l34_kafka-kafka-1 kafka-console-consumer.sh \
		--bootstrap-server localhost:9092 \
		--topic db \
		--from-beginning

new-user:
	curl localhost:3001/users -v -H 'Content-Type: application/json' \
		-d '{"name":"Tester CREATED","email":"tester@mail.tk"}'

put-user:
	curl localhost:3001/users/1 -v -X PUT -H 'Content-Type: application/json' \
		-d '{"name":"Tester UPDATED","email":"tester@mail.tk"}'

rm-user:
	curl localhost:3001/users/1 -v -X DELETE

new-product:
	curl localhost:3001/products -v -H 'Content-Type: application/json' \
		-d '{"name":"Testing CREATED","price":123}'

put-product:
	curl localhost:3001/products/1 -v -X PUT -H 'Content-Type: application/json' \
		-d '{"name":"Testing UPDATED","price":456}'

rm-product:
	curl localhost:3001/products/1 -v -X DELETE
