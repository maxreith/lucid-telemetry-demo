package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object EcuFaultSummary extends TelemetryJob {
  val name = "ecu_fault_summary"
  val description = "Faults and affected vehicles per ECU."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "diagnostic")
      .groupBy("payload.ecu")
      .agg(count(lit(1)).as("faults"), countDistinct("vin").as("affected_vins"))
  }
}
