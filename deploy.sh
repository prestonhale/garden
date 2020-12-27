docker build -t world .

aws ecr get-login-password --region us-west-2 | docker login --username AWS --password-stdin 764070331083.dkr.ecr.us-west-2.amazonaws.com
docker tag world:latest 764070331083.dkr.ecr.us-west-2.amazonaws.com/world
docker push 764070331083.dkr.ecr.us-west-2.amazonaws.com/world
aws ecs update-service --cluster world-cluster --service world-service --force-new-deployment > /dev/null

echo "Deploying!"