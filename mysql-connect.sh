sudo systemctl start docker
sleep 1
sudo docker exec -e IN_DOCKER -it csql-Opencart_dummy_DB-1 /bin/bash -c "mysql -pmauFJcuf5dhRMQrjj"