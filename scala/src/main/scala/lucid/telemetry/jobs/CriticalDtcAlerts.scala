package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object CriticalDtcAlerts extends TelemetryJob {
  val name = "critical_dtc_alerts"
  val description = "Diagnostic trouble codes with critical severity."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "diagnostic")
      .filter(col("payload.severity") === "critical")
      .select(col("vin"), col("event_id"), col("event_ts"), col("payload.code").as("code"), col("payload.ecu").as("ecu"))
  }
}
