#include "fmi3Functions.h"
#include <float.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_FRACTION 0.96
#define LOSS_FRACTION 0.02
#define STORAGE_FRACTION 0.02

typedef struct {
    fmi3Float64 time;
    fmi3Float64 pendingIntervalEnergyJ;
    fmi3Float64 serviceEnergyJ;
    fmi3Float64 lossEnergyJ;
    fmi3Float64 storageDeltaEnergyJ;
    fmi3Float64 cumulativeStoredEnergyJ;
    fmi3Boolean inputSet;
    fmi3Boolean initializationEntered;
    fmi3Boolean initialized;
    fmi3Boolean terminated;
    fmi3Boolean stopTimeDefined;
    fmi3Float64 stopTime;
} IntegratedFluxReceiver;

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
    if (strcmp(instantiationToken, "integrated-flux-receiver-token-v0.1") != 0) return NULL;
    return calloc(1, sizeof(IntegratedFluxReceiver));
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
    IntegratedFluxReceiver *receiver = instance;
    if (receiver == NULL || receiver->initializationEntered || receiver->initialized || receiver->terminated || !isfinite(startTime)
        || (toleranceDefined && (!isfinite(tolerance) || tolerance <= 0.0))
        || (stopTimeDefined && (!isfinite(stopTime) || stopTime <= startTime))) return fmi3Error;
    receiver->time = startTime;
    receiver->stopTimeDefined = stopTimeDefined;
    receiver->stopTime = stopTime;
    receiver->initializationEntered = fmi3True;
    receiver->pendingIntervalEnergyJ = 0.0;
    receiver->serviceEnergyJ = 0.0;
    receiver->lossEnergyJ = 0.0;
    receiver->storageDeltaEnergyJ = 0.0;
    receiver->cumulativeStoredEnergyJ = 0.0;
    receiver->inputSet = fmi3False;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3ExitInitializationMode(fmi3Instance instance) {
    IntegratedFluxReceiver *receiver = instance;
    if (receiver == NULL || !receiver->initializationEntered || receiver->initialized || receiver->terminated) return fmi3Error;
    receiver->initialized = fmi3True;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3SetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    const fmi3Float64 values[],
    size_t nValues) {
    IntegratedFluxReceiver *receiver = instance;
    if (receiver == NULL || !receiver->initialized || receiver->terminated
        || receiver->inputSet || valueReferences == NULL || values == NULL
        || nValueReferences != 1 || nValues != 1 || valueReferences[0] != 21
        || !isfinite(values[0]) || values[0] < 0.0) return fmi3Error;
    receiver->pendingIntervalEnergyJ = values[0];
    receiver->inputSet = fmi3True;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3GetFloat64(
    fmi3Instance instance,
    const fmi3ValueReference valueReferences[],
    size_t nValueReferences,
    fmi3Float64 values[],
    size_t nValues) {
    IntegratedFluxReceiver *receiver = instance;
    if (receiver == NULL || !receiver->initialized || receiver->terminated
        || valueReferences == NULL || values == NULL
        || nValueReferences != nValues || nValues == 0) return fmi3Error;
    for (size_t i = 0; i < nValueReferences; ++i) {
        switch (valueReferences[i]) {
            case 31: values[i] = receiver->serviceEnergyJ; break;
            case 32: values[i] = receiver->lossEnergyJ; break;
            case 33: values[i] = receiver->storageDeltaEnergyJ; break;
            case 34: values[i] = receiver->cumulativeStoredEnergyJ; break;
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
    IntegratedFluxReceiver *receiver = instance;
    (void)noSetFMUStatePriorToCurrentPoint;
    if (receiver == NULL || !receiver->initialized || receiver->terminated
        || !isfinite(currentCommunicationPoint) || !isfinite(communicationStepSize)
        || communicationStepSize <= 0.0 || currentCommunicationPoint != receiver->time
        || !receiver->inputSet || eventHandlingNeeded == NULL
        || terminateSimulation == NULL || earlyReturn == NULL
        || lastSuccessfulTime == NULL) return fmi3Error;

    const fmi3Float64 service = receiver->pendingIntervalEnergyJ * SERVICE_FRACTION;
    const fmi3Float64 loss = receiver->pendingIntervalEnergyJ * LOSS_FRACTION;
    const fmi3Float64 storage = receiver->pendingIntervalEnergyJ * STORAGE_FRACTION;
    const fmi3Float64 nextStoredEnergy = receiver->cumulativeStoredEnergyJ + storage;
    const fmi3Float64 nextTime = currentCommunicationPoint + communicationStepSize;
    const fmi3Float64 residual = receiver->pendingIntervalEnergyJ - service - loss - storage;
    if (!isfinite(service) || !isfinite(loss) || !isfinite(storage)
        || !isfinite(nextStoredEnergy) || !isfinite(nextTime)
        || nextTime <= currentCommunicationPoint
        || (receiver->stopTimeDefined && nextTime > receiver->stopTime)
        || !isfinite(residual)
        || fabs(residual) > 16.0 * DBL_EPSILON * fmax(receiver->pendingIntervalEnergyJ, 1.0)) {
        return fmi3Error;
    }

    receiver->serviceEnergyJ = service;
    receiver->lossEnergyJ = loss;
    receiver->storageDeltaEnergyJ = storage;
    receiver->cumulativeStoredEnergyJ = nextStoredEnergy;
    receiver->time = nextTime;
    receiver->inputSet = fmi3False;
    receiver->pendingIntervalEnergyJ = 0.0;
    *eventHandlingNeeded = fmi3False;
    *terminateSimulation = fmi3False;
    *earlyReturn = fmi3False;
    *lastSuccessfulTime = nextTime;
    return fmi3OK;
}

FMI3_Export fmi3Status fmi3Terminate(fmi3Instance instance) {
    IntegratedFluxReceiver *receiver = instance;
    if (receiver == NULL || !receiver->initialized || receiver->terminated) return fmi3Error;
    if (receiver->inputSet) return fmi3Error;
    receiver->terminated = fmi3True;
    return fmi3OK;
}
