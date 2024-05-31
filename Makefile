lint:
	.github/steps/lint.sh

bdd:
	cargo test

steps:
	make lint && make bdd