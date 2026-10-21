package com.jjthejjpro.monsters_app

import android.app.PendingIntent
import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.hardware.usb.UsbDevice
import android.hardware.usb.UsbDeviceConnection
import android.hardware.usb.UsbManager
import androidx.annotation.Keep
import androidx.core.content.ContextCompat
import com.hoho.android.usbserial.driver.CdcAcmSerialDriver
import com.hoho.android.usbserial.driver.ProbeTable
import com.hoho.android.usbserial.driver.UsbSerialDriver
import com.hoho.android.usbserial.driver.UsbSerialPort
import com.hoho.android.usbserial.driver.UsbSerialProber

@Keep
class AndroidSerialPort private constructor(context: Context) {
    private val context = context.applicationContext
    private val manager = this.context.getSystemService(Context.USB_SERVICE) as UsbManager
    private val permissionAction = "${this.context.packageName}.USB_PERMISSION"
    private val prober = UsbSerialProber(
        UsbSerialProber.getDefaultProbeTable()
            .addProduct(MONSTER_VENDOR_ID, MONSTER_PRODUCT_ID, CdcAcmSerialDriver::class.java)
            .addProduct(MONSTER_VENDOR_ID, ERROR_PRODUCT_ID, CdcAcmSerialDriver::class.java)
    )

    @Volatile
    private var listening = false

    @Volatile
    private var readGeneration = 0

    private var port: UsbSerialPort? = null
    private var connection: UsbDeviceConnection? = null
    @Volatile
    private var connectedDeviceName: String? = null
    @Volatile
    private var pendingDeviceName: String? = null
    private var deviceReceiverRegistered = false
    private var permissionReceiverRegistered = false

    private val deviceReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            when (intent.action) {
                UsbManager.ACTION_USB_DEVICE_ATTACHED -> scanForDevice()
                UsbManager.ACTION_USB_DEVICE_DETACHED -> {
                    val device = usbDeviceFrom(intent) ?: return
                    handleDeviceDetached(device)
                }
            }
        }
    }

    private val permissionReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            handlePermissionResult(intent)
        }
    }

    companion object {
        @SuppressLint("StaticFieldLeak")
        @Volatile
        private var instance: AndroidSerialPort? = null

        private const val MONSTER_VENDOR_ID = 0x4A4A
        private const val MONSTER_PRODUCT_ID = 0x4D4E
        private const val ERROR_PRODUCT_ID = 0x4D4D
        private const val BAUD_RATE = 115200
        private const val DATA_BITS = 8
        private const val READ_BUFFER_SIZE = 4096
        private const val READ_TIMEOUT_MS = 200
        private const val WRITE_TIMEOUT_MS = 1000

        fun initialize(context: Context) {
            System.loadLibrary("monsters_app_lib")
            if (instance == null) {
                synchronized(this) {
                    if (instance == null) {
                        instance = AndroidSerialPort(context.applicationContext).also {
                            it.nativeInitialize()
                        }
                    }
                }
            }
        }

        @Suppress("DEPRECATION")
        private fun usbDeviceFrom(intent: Intent): UsbDevice? =
            if (android.os.Build.VERSION.SDK_INT >= 33) {
                intent.getParcelableExtra(UsbManager.EXTRA_DEVICE, UsbDevice::class.java)
            } else {
                intent.getParcelableExtra(UsbManager.EXTRA_DEVICE)
            }
    }

    @Synchronized
    fun open() {
        if (listening) return
        listening = true

        try {
            ContextCompat.registerReceiver(
                context,
                deviceReceiver,
                IntentFilter().apply {
                    addAction(UsbManager.ACTION_USB_DEVICE_ATTACHED)
                    addAction(UsbManager.ACTION_USB_DEVICE_DETACHED)
                },
                ContextCompat.RECEIVER_EXPORTED
            )
            deviceReceiverRegistered = true
            ContextCompat.registerReceiver(
                context,
                permissionReceiver,
                IntentFilter(permissionAction),
                ContextCompat.RECEIVER_NOT_EXPORTED
            )
            permissionReceiverRegistered = true
            scanForDevice()
        } catch (error: Exception) {
            close()
            nativeOnError("Could not listen for USB serial devices: ${error.message}")
            throw error
        }
    }

    @Synchronized
    fun write(byte: Byte) {
        val currentPort = port ?: throw IllegalStateException("Serial port is not open")
        currentPort.write(byteArrayOf(byte), WRITE_TIMEOUT_MS)
    }

    @Synchronized
    fun close() {
        listening = false
        if (deviceReceiverRegistered) {
            context.unregisterReceiver(deviceReceiver)
            deviceReceiverRegistered = false
        }
        if (permissionReceiverRegistered) {
            context.unregisterReceiver(permissionReceiver)
            permissionReceiverRegistered = false
        }
        pendingDeviceName = null
        disconnect(notify = true)
    }

    @Synchronized
    private fun scanForDevice() {
        if (!listening || port != null || pendingDeviceName != null) return
        val driver = findDrivers().firstOrNull() ?: return
        requestConnection(driver)
    }

    private fun findDrivers(): List<UsbSerialDriver> =
        prober.findAllDrivers(manager)

    @Synchronized
    private fun requestConnection(driver: UsbSerialDriver) {
        val device = driver.device
        pendingDeviceName = device.deviceName
        nativeOnConnecting()
        if (manager.hasPermission(device)) {
            pendingDeviceName = null
            connect(driver)
            return
        }

        val permissionIntent = PendingIntent.getBroadcast(
            context,
            0,
            Intent(permissionAction).setPackage(context.packageName),
            PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
        manager.requestPermission(device, permissionIntent)
    }

    @Synchronized
    private fun handlePermissionResult(intent: Intent) {
        if (intent.action != permissionAction) return

        val deviceName = pendingDeviceName
        pendingDeviceName = null
        if (intent.getBooleanExtra(UsbManager.EXTRA_PERMISSION_GRANTED, false) && deviceName != null) {
            val driver = findDrivers().firstOrNull { it.device.deviceName == deviceName }
            if (driver != null) connect(driver) else nativeOnDisconnected()
        } else {
            nativeOnError("USB permission denied")
            nativeOnDisconnected()
        }
    }

    @Synchronized
    private fun handleDeviceDetached(device: UsbDevice) {
        when (device.deviceName) {
            connectedDeviceName -> {
                disconnect(notify = true)
                scanForDevice()
            }
            pendingDeviceName -> {
                pendingDeviceName = null
                nativeOnDisconnected()
                scanForDevice()
            }
        }
    }

    @Synchronized
    private fun connect(driver: UsbSerialDriver) {
        if (!listening || port != null) return
        val serialPort = driver.ports.firstOrNull()
        if (serialPort == null) {
            nativeOnError("USB device has no serial ports")
            nativeOnDisconnected()
            return
        }

        val deviceConnection = manager.openDevice(driver.device)
        if (deviceConnection == null) {
            nativeOnError("Could not open USB device")
            nativeOnDisconnected()
            return
        }

        try {
            serialPort.open(deviceConnection)
            serialPort.setParameters(
                BAUD_RATE,
                DATA_BITS,
                UsbSerialPort.STOPBITS_1,
                UsbSerialPort.PARITY_NONE
            )
        } catch (error: Exception) {
            try {
                serialPort.close()
            } catch (closeError: Exception) {
                nativeOnError("Could not close failed serial port: ${closeError.message}")
            }
            deviceConnection.close()
            nativeOnError("Could not configure serial port: ${error.message}")
            nativeOnDisconnected()
            return
        }

        port = serialPort
        connection = deviceConnection
        connectedDeviceName = driver.device.deviceName
        val generation = ++readGeneration
        nativeOnConnected(driver.device.productId == ERROR_PRODUCT_ID)
        startReading(serialPort, driver.device.deviceName, generation)
    }

    private fun startReading(serialPort: UsbSerialPort, deviceName: String, generation: Int) {
        Thread({
            val buffer = ByteArray(READ_BUFFER_SIZE)
            while (listening && generation == readGeneration) {
                try {
                    val count = serialPort.read(buffer, READ_TIMEOUT_MS)
                    if (manager.deviceList.values.none { it.deviceName == deviceName }) {
                        disconnect(notify = true)
                        return@Thread
                    }
                    for (index in 0 until count) {
                        nativeOnRead(buffer[index])
                    }
                } catch (error: Exception) {
                    if (listening && generation == readGeneration) {
                        nativeOnError("Serial read failed: ${error.message}")
                        disconnect(notify = true)
                    }
                    return@Thread
                }
            }
        }, "android-serial-reader").apply {
            isDaemon = true
            start()
        }
    }

    @Synchronized
    private fun disconnect(notify: Boolean) {
        val wasConnected = port != null
        readGeneration++
        val currentPort = port
        val currentConnection = connection
        port = null
        connection = null
        connectedDeviceName = null
        try {
            currentPort?.close()
        } catch (error: Exception) {
            nativeOnError("Could not close serial port: ${error.message}")
        } finally {
            currentConnection?.close()
        }
        if (notify && wasConnected) nativeOnDisconnected()
    }

    private external fun nativeInitialize()
    private external fun nativeOnConnecting()
    private external fun nativeOnConnected(errorMode: Boolean)
    private external fun nativeOnDisconnected()
    private external fun nativeOnRead(byte: Byte)
    private external fun nativeOnError(message: String)
}
