#include "fmi3Functions.h"
#include <dlfcn.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>

typedef fmi3InstantiateCoSimulationTYPE *Instantiate;
typedef fmi3FreeInstanceTYPE *FreeInstance;
typedef fmi3EnterInitializationModeTYPE *EnterInit;
typedef fmi3ExitInitializationModeTYPE *ExitInit;
typedef fmi3SetFloat64TYPE *SetFloat64;
typedef fmi3GetFloat64TYPE *GetFloat64;
typedef fmi3DoStepTYPE *DoStep;
typedef fmi3TerminateTYPE *Terminate;

#define LOAD(lib, variable, symbol, type) \
    type variable = (type)dlsym(lib, #symbol); \
    if (variable == NULL) { fprintf(stderr, "missing FMI symbol %s\\n", #symbol); return 2; }
#define CHECK(call) do { \
    fmi3Status status_ = (call); \
    if (status_ != fmi3OK) { \
        fprintf(stderr, "FMI call failed at %s:%d with status %d\\n", __FILE__, __LINE__, (int)status_); \
        return 3; \
    } \
} while (0)
#define NEAR(a, b) (fabs((a) - (b)) < 1e-9)

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: fmi3_smoke <source.so> <heat-exchanger.so>\\n");
        return 2;
    }
    void *geo = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    void *hx = dlopen(argv[2], RTLD_NOW | RTLD_LOCAL);
    if (geo == NULL || hx == NULL) {
        fprintf(stderr, "dlopen failed: %s\\n", dlerror());
        return 2;
    }
    LOAD(geo, g_inst, fmi3InstantiateCoSimulation, Instantiate)
    LOAD(geo, g_free, fmi3FreeInstance, FreeInstance)
    LOAD(geo, g_enter, fmi3EnterInitializationMode, EnterInit)
    LOAD(geo, g_exit, fmi3ExitInitializationMode, ExitInit)
    LOAD(geo, g_get, fmi3GetFloat64, GetFloat64)
    LOAD(geo, g_step, fmi3DoStep, DoStep)
    LOAD(geo, g_term, fmi3Terminate, Terminate)
    LOAD(hx, h_inst, fmi3InstantiateCoSimulation, Instantiate)
    LOAD(hx, h_free, fmi3FreeInstance, FreeInstance)
    LOAD(hx, h_enter, fmi3EnterInitializationMode, EnterInit)
    LOAD(hx, h_exit, fmi3ExitInitializationMode, ExitInit)
    LOAD(hx, h_set, fmi3SetFloat64, SetFloat64)
    LOAD(hx, h_get, fmi3GetFloat64, GetFloat64)
    LOAD(hx, h_step, fmi3DoStep, DoStep)
    LOAD(hx, h_term, fmi3Terminate, Terminate)

    fmi3Instance geothermal = g_inst("geothermal", "geo-source-token-v0.1", "",
        fmi3False, fmi3False, fmi3False, fmi3False, NULL, 0, NULL, NULL, NULL);
    fmi3Instance exchanger = h_inst("heat-exchanger", "heat-exchanger-token-v0.1", "",
        fmi3False, fmi3False, fmi3False, fmi3False, NULL, 0, NULL, NULL, NULL);
    if (geothermal == NULL || exchanger == NULL) { fprintf(stderr, "instantiate failed\\n"); return 3; }

    CHECK(g_enter(geothermal, fmi3False, 0.0, 0.0, fmi3True, 10.0));
    CHECK(h_enter(exchanger, fmi3False, 0.0, 0.0, fmi3True, 10.0));
    CHECK(g_exit(geothermal));
    CHECK(h_exit(exchanger));

    const fmi3ValueReference geo_ref[] = {11};
    const fmi3ValueReference input_ref[] = {21};
    const fmi3ValueReference output_refs[] = {31, 32, 33, 34};
    double total_input = 0.0, total_service = 0.0, total_loss = 0.0, total_storage = 0.0, stored = 0.0;

    for (int k = 0; k < 2; ++k) {
        const double t = (double)k;
        fmi3Boolean event = fmi3False, terminate = fmi3False, early = fmi3False;
        fmi3Float64 last = -1.0;
        CHECK(g_step(geothermal, t, 1.0, fmi3True, &event, &terminate, &early, &last));
        if (last != t + 1.0 || event || terminate || early) { fprintf(stderr, "source step mismatch\\n"); return 4; }
        double q = 0.0;
        CHECK(g_get(geothermal, geo_ref, 1, &q, 1));
        CHECK(h_set(exchanger, input_ref, 1, &q, 1));
        CHECK(h_step(exchanger, t, 1.0, fmi3True, &event, &terminate, &early, &last));
        double outputs[4] = {0};
        CHECK(h_get(exchanger, output_refs, 4, outputs, 4));
        if (outputs[0] != 480000.0 || outputs[1] != 10000.0
            || outputs[2] != 10000.0 || outputs[3] != (double)(k + 1) * 10000.0) {
            fprintf(stderr, "heat-exchanger output mismatch\\n");
            return 4;
        }
        total_input += q;
        total_service += outputs[0];
        total_loss += outputs[1];
        total_storage += outputs[2];
        stored = outputs[3];
    }

    if (total_input != total_service + total_loss + total_storage) {
        fprintf(stderr, "thermal conservation failed\\n");
        return 5;
    }

    const fmi3ValueReference bad_ref[] = {999};
    double bad_value = 0.0;
    if (g_get(geothermal, bad_ref, 1, &bad_value, 1) != fmi3Error) {
        fprintf(stderr, "undeclared value reference was not rejected\\n");
        return 6;
    }
    fmi3Boolean event = fmi3False, terminate = fmi3False, early = fmi3False;
    fmi3Float64 last = -99.0;
    if (g_step(geothermal, 1.5, 0.5, fmi3True, &event, &terminate, &early, &last) != fmi3Error) {
        fprintf(stderr, "out-of-order doStep was not rejected\\n");
        return 7;
    }

    CHECK(g_term(geothermal));
    CHECK(h_term(exchanger));
    g_free(geothermal);
    h_free(exchanger);
    dlclose(geo);
    dlclose(hx);

    printf("FMI 3.0.2 official-header direct C-API smoke: PASS\\n");
    printf("steps=2 input_J=%.0f service_J=%.0f loss_J=%.0f storage_J=%.0f residual_J=%.0f final_stored_J=%.0f\\n",
        total_input, total_service, total_loss, total_storage,
        total_input - total_service - total_loss - total_storage, stored);
    printf("negative_controls=unknown_value_reference,out_of_order_doStep: REJECTED\\n");
    return 0;
}
