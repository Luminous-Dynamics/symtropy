# KitchenModuleCapabilityV1

A versioned vendor-neutral description of module capabilities and their supporting evidence. This is not a device command schema, interlock protocol or certification.

- `KitchenModuleCapabilityV1.schema.json`: identity, module revision, action-group capabilities, operating envelope, interfaces, electrical profile, cleaning/service references, food-contact material pointers and safety authority reference.
- `fixtures/kitchen-module-capability-v1.json`: synthetic positive and adversarial fixtures deserialized by the Rust test suite.
- `src/module_capability.rs`: validation and action-group capability discovery.

## Key boundaries

- Capability declarations are not proof of physical fitment or interoperability. Mechanical/electrical/data interface references point to separate adapter records.
- Only real-power watts are stored as W. Apparent power, power factor, inrush waveform, protective device ratings and power quality need their own evidence before inverter qualification.
- Measured values require a calibration reference. Independent assay/qualification requires a method/protocol reference. Claim status cannot exceed its evidence class.
- Pressure-cooker modules require distinct pressure-vessel qualification, pressure-relief testing and independent report references. A generic datasheet or marketing claim is insufficient.
- The local safety authority is explicitly referenced but never controlled by Symtropy.
- Food-contact material and sanitation references are evidence pointers; they do not themselves prove regulatory compliance or hygiene.
- Capability discovery is advisory. It does not grant a module-control token or authorize a robot/appliance to execute.

All fixture records are synthetic; no physical module is qualified by this test suite.
