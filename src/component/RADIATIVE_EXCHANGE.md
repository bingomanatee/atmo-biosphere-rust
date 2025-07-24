# Radiative Exchnge Component

## Behavior: CalculateEnergyTransferBetween(cellA, cellB, simulationState)

## Input:
• cellA.temperatureInKelvin
• cellB.temperatureInKelvin
• cellA.emissivity
• cellB.emissivity
• simulationState.cellCenterDistanceInMeters
• simulationState.cellFaceAreaInSquareMeters
• simulationState.timeStepInSeconds
Output: energyTransfer (Joules) from A → B in this time step

# Steps:
1. averageTemperature = (cellA.temperatureInKelvin + cellB.temperatureInKelvin) ÷ 2
2. effectiveEmissivity = CalculateEffectiveEmissivity(cellA.emissivity, cellB.emissivity)
3. radiativeConductivity = CalculateRadiativeConductivity(
averageTemperature,
effectiveEmissivity,
simulationState.cellCenterDistanceInMeters
)
4. temperatureDifference = cellB.temperatureInKelvin – cellA.temperatureInKelvin
5. energyFluxRate = radiativeConductivity × temperatureDifference
• units: watts per square meter × square meters = watts
6. energyTransfer = energyFluxRate
× simulationState.cellFaceAreaInSquareMeters
× simulationState.timeStepInSeconds
7. return energyTransfer
