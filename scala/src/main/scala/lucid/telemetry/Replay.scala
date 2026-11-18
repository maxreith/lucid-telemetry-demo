package lucid.telemetry

import java.nio.charset.StandardCharsets
import java.nio.file.{Files, Paths}

import org.apache.spark.sql.SparkSession

/** Runs jobs in batch over recorded events and writes one JSONL file per job.
  *
  * Usage: Replay <events.jsonl> <output-dir> [job ...]   (no jobs = all jobs)
  */
object Replay {
  def main(args: Array[String]): Unit = {
    require(args.length >= 2, "usage: Replay <events.jsonl> <output-dir> [job ...]")
    val Array(input, outDir) = args.take(2)
    val requested = args.drop(2).toSeq
    val unknown = requested.filterNot(Jobs.byName.contains)
    require(unknown.isEmpty, s"unknown jobs: ${unknown.mkString(", ")}")
    val jobs = if (requested.isEmpty) Jobs.all else requested.map(Jobs.byName)

    val spark = SparkSession
      .builder()
      .appName("telemetry-replay")
      .master("local[*]")
      .config("spark.sql.session.timeZone", "UTC")
      .config("spark.sql.shuffle.partitions", "4")
      .config("spark.ui.enabled", "false")
      .getOrCreate()
    spark.sparkContext.setLogLevel("ERROR")

    val events = Events.load(spark, input).cache()
    Files.createDirectories(Paths.get(outDir))
    jobs.foreach { job =>
      val lines = job.run(events).toJSON.collect().sorted
      Files.write(Paths.get(outDir, s"${job.name}.jsonl"), (lines.mkString("\n") + "\n").getBytes(StandardCharsets.UTF_8))
      println(f"${job.name}%-28s ${lines.length}%6d records")
    }
    spark.stop()
  }
}
