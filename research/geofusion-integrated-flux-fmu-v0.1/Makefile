CC ?= cc
ZIP ?= zip
SOURCE_DATE_EPOCH ?= 315532800
BASE := ../geofusion-analytic-fmu-pair-v0.1
FMI_HEADERS := $(BASE)/third_party/FMI-3.0.2/headers
UNSUPPORTED := $(BASE)/src/fmi3_unsupported.c
BUILD := build
STAGE := $(BUILD)/stage
SSP := $(STAGE)/GeoFusion-Integrated-Flux-Federation.ssp
CFLAGS ?= -std=c11 -O2 -Wall -Wextra -Werror -fPIC -fvisibility=hidden -ffile-prefix-map=$(CURDIR)=.
LDFLAGS ?= -shared -Wl,--build-id=none

.PHONY: all package test clean
all: package test

$(BUILD)/IntegratedFluxSource.so: src/integrated_flux_source.c $(UNSUPPORTED) $(FMI_HEADERS)/fmi3Functions.h
	mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -I$(FMI_HEADERS) $(LDFLAGS) -o $@ $< $(UNSUPPORTED) -lm

$(BUILD)/IntegratedFluxReceiver.so: src/integrated_flux_receiver.c $(UNSUPPORTED) $(FMI_HEADERS)/fmi3Functions.h
	mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -I$(FMI_HEADERS) $(LDFLAGS) -o $@ $< $(UNSUPPORTED) -lm

package: $(BUILD)/IntegratedFluxSource.so $(BUILD)/IntegratedFluxReceiver.so
	rm -rf $(STAGE)
	mkdir -p $(STAGE)/source/binaries/x86_64-linux $(STAGE)/receiver/binaries/x86_64-linux
	cp model-description/integrated-flux-source.xml $(STAGE)/source/modelDescription.xml
	cp model-description/integrated-flux-receiver.xml $(STAGE)/receiver/modelDescription.xml
	cp $(BUILD)/IntegratedFluxSource.so $(STAGE)/source/binaries/x86_64-linux/IntegratedFluxSource.so
	cp $(BUILD)/IntegratedFluxReceiver.so $(STAGE)/receiver/binaries/x86_64-linux/IntegratedFluxReceiver.so
	cp src/integrated_flux_source.c $(STAGE)/source/
	cp src/integrated_flux_receiver.c $(STAGE)/receiver/
	cp $(UNSUPPORTED) $(STAGE)/source/fmi3_unsupported.c
	cp $(UNSUPPORTED) $(STAGE)/receiver/fmi3_unsupported.c
	cp $(BASE)/third_party/FMI-3.0.2/LICENSE.txt $(STAGE)/source/FMI-3.0.2-LICENSE.txt
	cp $(BASE)/third_party/FMI-3.0.2/LICENSE.txt $(STAGE)/receiver/FMI-3.0.2-LICENSE.txt
	find $(STAGE) -type f -exec touch -d @$(SOURCE_DATE_EPOCH) {} +
	cd $(STAGE)/source && find . -type f -printf '%P\n' | LC_ALL=C sort | $(ZIP) -X -q ../GeoFusion-IntegratedFluxSource.fmu -@
	cd $(STAGE)/receiver && find . -type f -printf '%P\n' | LC_ALL=C sort | $(ZIP) -X -q ../GeoFusion-IntegratedFluxReceiver.fmu -@
	cd $(STAGE) && sha256sum *.fmu > artifact-digests.sha256
	python3 ssp/integrated_flux_ssp.py $(STAGE)/GeoFusion-IntegratedFluxSource.fmu $(STAGE)/GeoFusion-IntegratedFluxReceiver.fmu $(SSP)
	cd $(STAGE) && sha256sum *.ssp >> artifact-digests.sha256
	cd $(BUILD) && sha256sum IntegratedFluxSource.so IntegratedFluxReceiver.so >> stage/artifact-digests.sha256

test: package
	python3 tests/check_fmi3_surface.py
	python3 tests/test_integrated_flux_ssp.py
	python3 tests/fmpy_flux_smoke.py

clean:
	rm -rf $(BUILD)
