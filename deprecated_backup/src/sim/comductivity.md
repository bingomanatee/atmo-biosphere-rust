## A. Full‐name formula for the conductance coefficient between cell i and neighbor j
For each cell i, you first compute its pressure-adjusted thermal conductivity:

```
pressure_difference = cell.pressure_pa - reference_pressure_pa
pressure_difference = clamp(pressure_difference, minimum_allowed_pressure_difference, maximum_allowed_pressure_difference)

beta_coefficient = cell.thermal_expansivity_per_kelvin * cell.bulk_modulus_pa

pressure_adjusted_conductivity =
cell.base_thermal_conductivity_watts_per_meter_kelvin
* cell.thermal_conduction_modifier
* (1.0 + beta_coefficient * (pressure_difference / cell.bulk_modulus_pa))
  Then for each neighbor j you compute:

interface_conductivity_watts_per_meter_kelvin =
(pressure_adjusted_conductivity_of_cell_i
+ pressure_adjusted_conductivity_of_cell_j)
  / 2.0

shared_contact_area_square_meters =
if neighbor_is_above_or_below {
cell.horizontal_footprint_area_square_meters
} else {
shared_edge_length_meters * cell.vertical_cell_thickness_meters
}

center_to_center_distance_meters =
if neighbor_is_above_or_below {
cell.vertical_cell_thickness_meters
} else {
horizontal_center_to_center_distance_meters
}

conductance_coefficient_joules_per_kelvin =
interface_conductivity_watts_per_meter_kelvin
* shared_contact_area_square_meters
  / center_to_center_distance_meters
* simulation_timestep_seconds
  At runtime, the heat exchanged from i to j is simply:
delta_energy_joules = conductance_coefficient_joules_per_kelvin
* (temperature_of_j_kelvin - temperature_of_i_kelvin)
```

## B. Which quantities come from your material constants vs. which are sub-calculations
Quantity	Source

From material constants

* base_thermal_conductivity_watts_per_meter_kelvin	materials.json / Material │
* thermal_conduction_modifier	materials.json / MaterialPhase │
* bulk_modulus_pa	materials.json / Material │
* thermal_expansivity_per_kelvin	materials.json / Material │
From simulation or geometry setup
* reference_pressure_pa (e.g. 1 × 10⁵ Pa)	constant you choose │
* minimum_allowed_pressure_difference, maximum_allowed_pressure_difference	constants you choose
* simulation_timestep_seconds	simulation config │
* shared_edge_length_meters	computed from H3 resolution │
* horizontal_center_to_center_distance_meters	computed from H3 resolution │
* cell.horizontal_footprint_area_square_meters	computed from H3 resolution │
* cell.vertical_cell_thickness_meters	your layer thickness │
Sub-calculations during precompute
* pressure_difference	pressure_pa − reference_pressure_pa; clamped │
* beta_coefficient	thermal_expansivity_per_kelvin × bulk_modulus_pa │
* pressure_adjusted_conductivity	combines base conductivity, modifier, and β ⋅ ΔP/K │
* interface_conductivity_watts_per_meter_kelvin	average of the two neighboring conductivities │
* shared_contact_area_square_meters	based on neighbor orientation and cell footprint │
* center_to_center_distance_meters	based on neighbor orientation │
* conductance_coefficient_joules_per_kelvin	final Gᵢⱼ value you store │