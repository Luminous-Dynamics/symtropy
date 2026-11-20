CC ?= cc
ZIP ?= zip
SOURCE_DATE_EPOCH ?= 315532800
FMI_HEADERS := third_party/FMI-3.0.2/headers
BUILD := build
STAGE := $(BUILD)/stage
CFLAGS ?= -std=c11 -O2 -Wall -Wextra -Werror -fPIC -fvisibility=hidden -ffile-prefix-map=$(CURDIR)=.
LDFLAGS ?= -shared -Wl,--build-id=none
COMMON_HEADERS := $(FMI_HEADERS)/fmi3Functions.h $(FMI_HEADERS)/fmi3FunctionTypes.h $(FMI_HEADERS)/fmi3PlatformTypes.h

.PHONY: all package test clean
all: package test

$(BUILD)/GeothermalSource.so: src/geothermal_source.c src/fmi3_unsupported.c $(COMMON_HEADERS)
	mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -I$(FMI_HEADERS) $(LDFLAGS) -o $@ src/geothermal_source.c src/fmi3_unsupported.c -lm

$(BUILD)/HeatExchangerBuffer.so: src/heat_exchanger_buffer.c src/fmi3_unsupported.c $(COMMON_HEADERS)
	mkdir -p $(BUILD)
	$(CC) $(CFLAGS) -I$(FMI_HEADERS) $(LDFLAGS) -o $@ src/heat_exchanger_buffer.c src/fmi3_unsupported.c -lm

$(BUILD)/fmi3_smoke: tests/fmi3_smoke.c $(COMMON_HEADERS)
	mkdir -p $(BUILD)
	$(CC) -std=c11 -O2 -Wall -Wextra -Werror -I$(FMI_HEADERS) -o $@ $< -ldl -lm

package: $(BUILD)/GeothermalSource.so $(BUILD)/HeatExchangerBuffer.so
	rm -rf $(STAGE)
	mkdir -p $(STAGE)/geothermal/binaries/x86_64-linux $(STAGE)/geothermal/sources $(STAGE)/geothermal/licenses $(STAGE)/geothermal/resources
	mkdir -p $(STAGE)/heat-exchanger/binaries/x86_64-linux $(STAGE)/heat-exchanger/sources $(STAGE)/heat-exchanger/licenses $(STAGE)/heat-exchanger/resources
	cp model-description/geothermal-source.xml $(STAGE)/geothermal/modelDescription.xml
	cp model-description/heat-exchanger-buffer.xml $(STAGE)/heat-exchanger/modelDescription.xml
	cp resources/README.txt $(STAGE)/geothermal/resources/README.txt
	cp resources/README.txt $(STAGE)/heat-exchanger/resources/README.txt
	cp $(BUILD)/GeothermalSource.so $(STAGE)/geothermal/binaries/x86_64-linux/GeothermalSource.so
	cp $(BUILD)/HeatExchangerBuffer.so $(STAGE)/heat-exchanger/binaries/x86_64-linux/HeatExchangerBuffer.so
	cp src/geothermal_source.c src/fmi3_unsupported.c $(STAGE)/geothermal/sources/
	cp src/heat_exchanger_buffer.c src/fmi3_unsupported.c $(STAGE)/heat-exchanger/sources/
	cp $(COMMON_HEADERS) $(STAGE)/geothermal/sources/
	cp $(COMMON_HEADERS) $(STAGE)/heat-exchanger/sources/
	cp third_party/FMI-3.0.2/LICENSE.txt $(STAGE)/geothermal/licenses/FMI-3.0.2-LICENSE.txt
	cp third_party/FMI-3.0.2/LICENSE.txt $(STAGE)/heat-exchanger/licenses/FMI-3.0.2-LICENSE.txt
	find $(STAGE) -type f -exec touch -d @$(SOURCE_DATE_EPOCH) {} +
	cd $(STAGE)/geothermal && find . -type f -printf '%P
' | LC_ALL=C sort | $(ZIP) -X -q ../GeoFusion-GeothermalSource.fmu -@
	cd $(STAGE)/heat-exchanger && find . -type f -printf '%P
' | LC_ALL=C sort | $(ZIP) -X -q ../GeoFusion-HeatExchangerBuffer.fmu -@
	cd $(STAGE) && sha256sum *.fmu > artifact-digests.sha256
	cd $(BUILD) && sha256sum GeothermalSource.so HeatExchangerBuffer.so >> stage/artifact-digests.sha256

test: package $(BUILD)/fmi3_smoke
	$(BUILD)/fmi3_smoke $(STAGE)/geothermal/binaries/x86_64-linux/GeothermalSource.so $(STAGE)/heat-exchanger/binaries/x86_64-linux/HeatExchangerBuffer.so
	python3 tests/analytic_oracle.py $(STAGE)/GeoFusion-GeothermalSource.fmu $(STAGE)/GeoFusion-HeatExchangerBuffer.fmu

clean:
	rm -rf $(BUILD)
