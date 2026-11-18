ThisBuild / scalaVersion := "2.13.15"

lazy val root = (project in file("."))
  .settings(
    name := "telemetry-jobs",
    libraryDependencies += "org.apache.spark" %% "spark-sql" % "3.5.3",
    fork := true,
    outputStrategy := Some(StdoutOutput),
    javaOptions ++= Seq(
      "java.lang", "java.lang.invoke", "java.lang.reflect", "java.io", "java.net", "java.nio",
      "java.util", "java.util.concurrent", "java.util.concurrent.atomic"
    ).map(p => s"--add-opens=java.base/$p=ALL-UNNAMED") ++ Seq(
      "sun.nio.ch", "sun.nio.cs", "sun.security.action", "sun.util.calendar"
    ).map(p => s"--add-opens=java.base/$p=ALL-UNNAMED") ++ Seq("-Xmx2g")
  )
