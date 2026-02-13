import 'package:flutter/material.dart';

class StatusIndicator extends StatelessWidget {
  final bool isConnected;

  const StatusIndicator({
    super.key,
    required this.isConnected,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 12,
      height: 12,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: isConnected ? const Color(0xFF00FF88) : const Color(0xFFFF8800),
        boxShadow: [
          BoxShadow(
            color: (isConnected ? const Color(0xFF00FF88) : const Color(0xFFFF8800))
                .withOpacity(0.5),
            blurRadius: 8,
            spreadRadius: 2,
          ),
        ],
      ),
    );
  }
}
