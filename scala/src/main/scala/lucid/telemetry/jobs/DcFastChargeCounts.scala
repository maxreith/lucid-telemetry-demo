package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object DcFastChargeCounts extends TelemetryJob {
  val name = "dc_fast_charge_counts"
  val description = "Number of DC fast-charging sessions per vehicle."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "charging_start")
      .filter(col("payload.charger_type") === "DC")
      .groupBy("vin")
      .agg(count(lit(1)).as("dc_sessions"))
  }
}
