package lucid.telemetry.jobs

import lucid.telemetry.{Events, TelemetryJob}
import org.apache.spark.sql.DataFrame
import org.apache.spark.sql.functions._

object DtcFirstSeen extends TelemetryJob {
  val name = "dtc_first_seen"
  val description = "First and last time each diagnostic trouble code was seen per vehicle."

  def run(events: DataFrame): DataFrame = {
    Events.ofType(Events.dedup(events), "diagnostic")
      .groupBy("vin", "payload.code")
      .agg(min("event_ts").as("first_seen_ts"), max("event_ts").as("last_seen_ts"))
  }
}
