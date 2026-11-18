package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object TirePressureAlerts extends TelemetryJob {
  val name = "tire_pressure_alerts"
  val description = "One alert per tire reading below 200 kPa."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "tire_pressure")
      .select(
        col("vin"), col("event_id"), col("event_ts"),
        explode(array(Seq("fl", "fr", "rl", "rr").map(t =>
          struct(lit(t).as("tire"), col(s"payload.$t").as("pressure_kpa"))): _*)).as("reading")
      )
      .filter(col("reading.pressure_kpa") < 200.0)
      .select(col("vin"), col("event_id"), col("event_ts"), col("reading.tire").as("tire"),
        col("reading.pressure_kpa").as("pressure_kpa"))
  }
}
