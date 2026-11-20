#include "fmi3Functions.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    fmi3Float64 time;
    fmi3Float64 inputPower;
    fmi3Float64 servicePower;
    fmi3Float64 lossPower;
    fmi3Float64 storagePower;
    fmi3Float64 storedEnergy;
    fmi3Boolean initialized;
    fmi3Boolean terminated;
} HeatExchangerBuffer;

FMI3_Export const char *fmi3GetVersion(void) {
    return "3.0.2";
}

FMI3_Export fmi3Instance fmi3InstantiateCoSimulation(
    fmi3String instanceName,
    fmi3String instantiationToken,
    fmi3String resourcePath,
    fmi3Boolean visible,
    fmi3Boolean loggingOn,
    fmi3Boolean eventModeUsed,
    fmi3Boolean earlyReturnAllowed,
    const fmi3ValueReference requiredIntermediateVariables[],
    size_t nRequiredIntermediateVariables,
    fmi3InstanceEnvironment instanceEnvironment,
    fmi3LogMessageCallback logMessage,
    fmi3IntermediateUpdateCallback intermediateUpdate) {
    (void)resourcePath;
    (void)loggingOn;
    (void)eventModeUsed;
    (void)earlyReturnAllowed;
    (void)requiredIntermediateVariables;
    (void)nRequiredIntermediateVariables;
    (void)instanceEnvironment;
    (void)logMessage;
    (void)intermediateUpdate;
    if (instanceName == NULL || instantiationToken == NULL || visible) return NULL;
    if (strcmp(instantiationToken, "heat-exchanger-token-v0.1") != 0) return NULL;
    return calloc(1, sizeof(HeatExchangerBuffer));
}

FMI3_Export void fmi3FreeInstance(fmi3Instance instance) {
    free(instance);
}

FMI3_Export fmi3Status fmi3EnterInitializationMode(
    fmi3Instance instance,
    fmi3Boolean toleranceDefined,
    fmi3Float64 tolerance,
    fmi3Float64 startTime,
    fmi3Boolean stopTimeDefined,
    fmi3Float64 stopTime) {
    HeatExchangerBuffer *buffer = instance;
    if (buffer == NULL || buffer->initialized || buffer->terminated || !isfinite(startTime)
        || (toleranceDefined && (!isfinite(tolerance) || tolerance <= 0.0))
        || (stopTimeDefined && (!isfinite(stopTime) || stopTime <= startTime))) return fmi3Error;
    buffer->time = startTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3ExitInitializationMode(fmi3Instance instance) {
    HeatExchangerBuffer *buffer = instance;
    if (buffer == NULL || buffer->initialized || buffer->terminated) return fmi3Error;
    buffer->initialized = fmi3True;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3SetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    const fmi3Float64 values[],
    size_t nValues) {
    HeatExchangerBuffer *buffer = instance;
    if (buffer == NULL || !buffer->initialized || buffer->terminated || valueReferences == NULL
        || values == NULL || nValueReferences != 1 || nValues != 1 || valueReferences[0] != 21
        || !isfinite(values[0]) || values[0] < 0.0) return fmi3Error;
    buffer->inputPower = values[0];
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3GetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    fmi3Float64 values[],
    size_t nValues) {
    HeatExchangerBuffer *buffer = instance;
    if (buffer == NULL || !buffer->initialized || buffer->terminated || valueReferences == NULL
        || values == NULL || nValueReferences != nValues || nValues == 0) return fmi3Error;
    for (size_t i = 0; i < nValueReferences; ++i) {
        switch (valueReferences[i]) {
            case 31: values[i] = buffer->servicePower; break;
            case 32: values[i] = buffer->lossPower; break;
            case 33: values[i] = buffer->storagePower; break;
            case 34: values[i] = buffer->storedEnergy; break;
            default: return fmi3Error;
        }
    }
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3DoStep(
    fmi3Instance instance,
    fmi3Float64 currentCommunicationPoint,
    fmi3Float64 communicationStepSize,
    fmi3Boolean noSetFMUStatePriorToCurrentPoint,
    fmi3Boolean *eventHandlingNeeded,
    fmi3Boolean *terminateSimulation,
    fmi3Boolean *earlyReturn,
    fmi3Float64 *lastSuccessfulTime) {
    HeatExchangerBuffer *buffer = instance;
    (void)noSetFMUStatePriorToCurrentPoint;
    if (buffer == NULL || !buffer->initialized || buffer->terminated
        || !isfinite(currentCommunicationPoint) || !isfinite(communicationStepSize)
        || communicationStepSize <= 0.0 || currentCommunicationPoint != buffer->time
        || eventHandlingNeeded == NULL || terminateSimulation == NULL || earlyReturn == NULL
        || lastSuccessfulTime == NULL) return fmi3Error;

    const fmi3Float64 service = buffer->inputPower * 0.96;
    const fmi3Float64 loss = buffer->inputPower * 0.02;
    const fmi3Float64 storage = buffer->inputPower * 0.02;
    const fmi3Float64 nextEnergy = buffer->storedEnergy + storage * communicationStepSize;
    const fmi3Float64 nextTime = currentCommunicationPoint + communicationStepSize;
    if (!isfinite(service) || !isfinite(loss) || !isfinite(storage)
        || !isfinite(nextEnergy) || !isfinite(nextTime)) return fmi3Error;

    buffer->servicePower = service;
    buffer->lossPower = loss;
    buffer->storagePower = storage;
    buffer->storedEnergy = nextEnergy;
    buffer->time = nextTime;
    *eventHandlingNeeded = fmi3False;
    *terminateSimulation = fmi3False;
    *earlyReturn = fmi3False;
    *lastSuccessfulTime = nextTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3Terminate(fmi3Instance instance) {
    HeatExchangerBuffer *buffer = instance;
    if (buffer == NULL || !buffer->initialized || buffer->terminated) return fmi3Error;
    buffer->terminated = fmi3True;
    return fmi3OK;
}
