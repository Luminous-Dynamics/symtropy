#include "fmi3Functions.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define MEAN_POWER_W 500000.0
#define AMPLITUDE_W 100000.0
#define ANGULAR_FREQUENCY_RAD_S (2.0 * 3.14159265358979323846 / 400.0)

typedef struct {
    fmi3Float64 time;
    fmi3Float64 cumulativeEnergyJ;
    fmi3Float64 intervalEnergyJ;
    fmi3Boolean initializationEntered;
    fmi3Boolean initialized;
    fmi3Boolean terminated;
    fmi3Boolean stopTimeDefined;
    fmi3Float64 stopTime;
} IntegratedFluxSource;

static fmi3Float64 stable_sinc(fmi3Float64 x) {
    const fmi3Float64 x2 = x * x;
    if (fabs(x) < 1.0e-4) {
        return 1.0 - x2 / 6.0 + x2 * x2 / 120.0 - x2 * x2 * x2 / 5040.0;
    }
    return sin(x) / x;
}

static fmi3Float64 integrate_source_energy(fmi3Float64 start_s, fmi3Float64 end_s) {
    const fmi3Float64 duration = end_s - start_s;
    const fmi3Float64 midpoint = start_s + duration * 0.5;
    const fmi3Float64 half_phase = ANGULAR_FREQUENCY_RAD_S * duration * 0.5;
    return MEAN_POWER_W * duration
        + AMPLITUDE_W * duration
            * sin(ANGULAR_FREQUENCY_RAD_S * midpoint)
            * stable_sinc(half_phase);
}

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
    if (instanceName == NULL || instantiationToken == NULL || visible
        || eventModeUsed || nRequiredIntermediateVariables != 0) return NULL;
    if (strcmp(instantiationToken, "integrated-flux-source-token-v0.1") != 0) return NULL;
    return calloc(1, sizeof(IntegratedFluxSource));
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
    IntegratedFluxSource *source = instance;
    if (source == NULL || source->initializationEntered || source->initialized || source->terminated || !isfinite(startTime)
        || (toleranceDefined && (!isfinite(tolerance) || tolerance <= 0.0))
        || (stopTimeDefined && (!isfinite(stopTime) || stopTime <= startTime))) return fmi3Error;
    source->time = startTime;
    source->stopTimeDefined = stopTimeDefined;
    source->stopTime = stopTime;
    source->cumulativeEnergyJ = 0.0;
    source->intervalEnergyJ = 0.0;
    source->initializationEntered = fmi3True;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3ExitInitializationMode(fmi3Instance instance) {
    IntegratedFluxSource *source = instance;
    if (source == NULL || !source->initializationEntered || source->initialized || source->terminated) return fmi3Error;
    source->initialized = fmi3True;
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

FMI3_Export fmi3Status fmi3GetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    fmi3Float64 values[],
    size_t nValues) {
    IntegratedFluxSource *source = instance;
    if (source == NULL || !source->initialized || source->terminated
        || valueReferences == NULL || values == NULL
        || nValueReferences != nValues || nValues == 0) return fmi3Error;
    for (size_t i = 0; i < nValueReferences; ++i) {
        switch (valueReferences[i]) {
            case 11: values[i] = source->cumulativeEnergyJ; break;
            case 12: values[i] = source->intervalEnergyJ; break;
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
    IntegratedFluxSource *source = instance;
    (void)noSetFMUStatePriorToCurrentPoint;
    if (source == NULL || !source->initialized || source->terminated
        || !isfinite(currentCommunicationPoint) || !isfinite(communicationStepSize)
        || communicationStepSize <= 0.0 || currentCommunicationPoint != source->time
        || eventHandlingNeeded == NULL || terminateSimulation == NULL
        || earlyReturn == NULL || lastSuccessfulTime == NULL) return fmi3Error;

    const fmi3Float64 nextTime = currentCommunicationPoint + communicationStepSize;
    if (!isfinite(nextTime) || nextTime <= currentCommunicationPoint
        || (source->stopTimeDefined && nextTime > source->stopTime)) return fmi3Error;
    const fmi3Float64 intervalEnergy = integrate_source_energy(currentCommunicationPoint, nextTime);
    const fmi3Float64 nextCumulativeEnergy = source->cumulativeEnergyJ + intervalEnergy;
    if (!isfinite(intervalEnergy) || intervalEnergy < 0.0
        || !isfinite(nextCumulativeEnergy) || nextCumulativeEnergy < source->cumulativeEnergyJ) {
        return fmi3Error;
    }

    source->intervalEnergyJ = intervalEnergy;
    source->cumulativeEnergyJ = nextCumulativeEnergy;
    source->time = nextTime;
    *eventHandlingNeeded = fmi3False;
    *terminateSimulation = fmi3False;
    *earlyReturn = fmi3False;
    *lastSuccessfulTime = nextTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3Terminate(fmi3Instance instance) {
    IntegratedFluxSource *source = instance;
    if (source == NULL || !source->initialized || source->terminated) return fmi3Error;
    source->terminated = fmi3True;
    return fmi3OK;
}
