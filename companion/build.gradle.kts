plugins {
    id("dev.kikugie.loom-back-compat")
}

version = "${property("mod.version")}+${sc.current.version}"
base.archivesName = property("mod.id") as String

val requiredJava: JavaVersion = when {
    sc.current.parsed >= "26.1" -> JavaVersion.VERSION_25
    sc.current.parsed >= "1.20.5" -> JavaVersion.VERSION_21
    else -> JavaVersion.VERSION_17
}

dependencies {
    minecraft("com.mojang:minecraft:${sc.current.version}")
    loomx.applyMojangMappings()
    modImplementation("net.fabricmc:fabric-loader:${property("deps.fabric_loader")}")
    val fabricApiVersion = sc.properties.get<String>("deps.fabric_api")
    for (module in sc.properties.get<String>("deps.fabric_api_modules").split(",")) {
        val dependency = fabricApi.module(module.trim(), fabricApiVersion)
        modImplementation(dependency)
        include(dependency)
    }
}

loom {
    fabricModJsonPath = rootProject.file("src/main/resources/fabric.mod.json")
    runConfigs.all {
        generateRunConfig = false
    }
}

java {
    targetCompatibility = requiredJava
    sourceCompatibility = requiredJava
    toolchain {
        vendor = JvmVendorSpec.ADOPTIUM
        languageVersion = JavaLanguageVersion.of(requiredJava.majorVersion)
    }
}

tasks {
    processResources {
        val props = mapOf(
            "id" to sc.properties["mod.id"],
            "name" to sc.properties["mod.name"],
            "version" to sc.properties["mod.version"],
            "minecraft" to sc.properties["mod.mc_compat"],
            "loader" to Regex("\\d+\\.\\d+").find(sc.properties.get<String>("deps.fabric_loader"))!!.value,
        )
        inputs.properties(props)
        filesMatching("fabric.mod.json") { expand(props) }
    }

    val minecraftVersion = sc.current.version
    register<Copy>("collectJar") {
        group = "build"
        description = "Copies the mod jar to build/libs/companion as orrery-companion-<minecraft>.jar"
        inputs.property("minecraft", minecraftVersion)
        from(loomx.modJar.flatMap { it.archiveFile })
        into(rootProject.layout.buildDirectory.dir("libs/companion"))
        rename { "orrery-companion-$minecraftVersion.jar" }
    }
}
