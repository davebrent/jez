WEBPACK = ./node_modules/.bin/webpack-dev-server

# For reloading navigate to http://0.0.0.0:8000/webpack-dev-server
# Note. Serves the bundle entirely from memory
develop:
	@$(WEBPACK) \
		--resolve-alias="osc-js=./node_modules/osc-js/lib/osc.browser.js" \
		--watch-poll=1000 \
		--mode=development \
		--progress \
		--entry=./online.js \
		--output=bundle.js \
		--port=8000 \
		--host=0.0.0.0
