PYTHON ?= python3

.PHONY: daemon-inventory ui-mock ui-real dev-ui

daemon-inventory:
	cargo run --manifest-path daemon/Cargo.toml -- inventory

ui-mock:
	cd app && PH18_UI_BACKEND=mock $(PYTHON) -m ph18_72_lighting_ui.main

ui-real:
	cd app && PH18_UI_BACKEND=cargo $(PYTHON) -m ph18_72_lighting_ui.main

dev-ui:
	./scripts/dev-ui.sh
