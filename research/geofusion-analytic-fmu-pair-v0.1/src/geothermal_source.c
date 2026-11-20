#include "fmi3Functions.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    fmi3Float64 time;
    fmi3Boolean initialized;
    fmi3Boolean terminated;
} GeothermalSource;

FMI3_Export const char *fmi3GetVersion(void) {
    return "3.0";
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
    if (strcmp(instantiationToken, "geo-source-token-v0.1") != 0) return NULL;
    return calloc(1, sizeof(GeothermalSource));
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
    GeothermalSource *source = instance;
    if (source == NULL || source->initialized || source->terminated || !isfinite(startTime)
        || (toleranceDefined && (!isfinite(tolerance) || tolerance <= 0.0))
        || (stopTimeDefined && (!isfinite(stopTime) || stopTime <= startTime))) return fmi3Error;
    source->time = startTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3ExitInitializationMode(fmi3Instance instance) {
    GeothermalSource *source = instance;
    if (source == NULL || source->initialized || source->terminated) return fmi3Error;
    source->initialized = fmi3True;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3GetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    fmi3Float64 values[],
    size_t nValues) {
    GeothermalSource *source = instance;
    if (source == NULL || !source->initialized || source->terminated || valueReferences == NULL
        || values == NULL || nValueReferences != 1 || nValues != 1) return fmi3Error;
    if (valueReferences[0] != 11) return fmi3Error;
    values[0] = 500000.0;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3SetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    const fmi3Float64 values[],
    size_t nValues) {
    (void)instance;
    (void)valueReferences;
    (void)nValueReferences;
    (void)values;
    (void)nValues;
    return fmi3Error;
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
    GeothermalSource *source = instance;
    (void)noSetFMUStatePriorToCurrentPoint;
    if (source == NULL || !source->initialized || source->terminated
        || !isfinite(currentCommunicationPoint) || !isfinite(communicationStepSize)
        || communicationStepSize <= 0.0 || currentCommunicationPoint != source->time
        || eventHandlingNeeded == NULL || terminateSimulation == NULL || earlyReturn == NULL
        || lastSuccessfulTime == NULL) return fmi3Error;
    const fmi3Float64 nextTime = currentCommunicationPoint + communicationStepSize;
    if (!isfinite(nextTime)) return fmi3Error;
    source->time = nextTime;
    *eventHandlingNeeded = fmi3False;
    *terminateSimulation = fmi3False;
    *earlyReturn = fmi3False;
    *lastSuccessfulTime = nextTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3Terminate(fmi3Instance instance) {
    GeothermalSource *source = instance;
    if (source == NULL || !source->initialized || source->terminated) return fmi3Error;
    source->terminated = fmi3True;
    return fmi3OK;
}
