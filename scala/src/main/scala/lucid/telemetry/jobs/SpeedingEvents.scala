package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object SpeedingEvents extends TelemetryJob {
  val name = "speeding_events"
  val description = "Location pings above 120 km/h."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "location")
      .filter(col("payload.speed_kph") > 120.0)
      .select(col("vin"), col("event_id"), col("event_ts"), col("payload.speed_kph").as("speed_kph"))
  }
}
