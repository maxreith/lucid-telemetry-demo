package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object AdasEventCounts extends TelemetryJob {
  val name = "adas_event_counts"
  val description = "Driver-assistance events per vehicle and kind."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "adas_event")
      .groupBy("vin", "payload.kind")
      .agg(count(lit(1)).as("events"))
  }
}
